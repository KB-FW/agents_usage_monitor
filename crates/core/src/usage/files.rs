//! File discovery and incremental JSONL reading.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub mtime: i64,
}

/// `(dir name, depth, full path) -> skip?`
pub type SkipDir<'a> = &'a dyn Fn(&str, usize, &Path) -> bool;

pub struct ListOptions<'a> {
    pub matches: &'a dyn Fn(&str) -> bool,
    pub skip_dir: Option<SkipDir<'a>>,
    pub modified_since: i64,
    pub max_depth: usize,
}

/// Recursively list files under `root` (missing roots simply yield nothing).
pub fn list_files(root: &Path, opts: &ListOptions) -> Vec<FileInfo> {
    let mut out = Vec::new();
    walk(root, 0, opts, &mut out);
    out
}

fn walk(dir: &Path, depth: usize, opts: &ListOptions, out: &mut Vec<FileInfo>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for ent in entries.flatten() {
        let path = ent.path();
        let name = ent.file_name().to_string_lossy().to_string();
        let Ok(ft) = ent.file_type() else { continue };
        let is_dir = ft.is_dir() || (ft.is_symlink() && path.is_dir());
        if is_dir {
            if depth >= opts.max_depth || name == "node_modules" || name == ".git" {
                continue;
            }
            if let Some(skip) = opts.skip_dir {
                if skip(&name, depth, &path) {
                    continue;
                }
            }
            walk(&path, depth + 1, opts, out);
        } else if (opts.matches)(&name) {
            let Ok(meta) = fs::metadata(&path) else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            if mtime < opts.modified_since {
                continue;
            }
            out.push(FileInfo {
                path,
                size: meta.len(),
                mtime,
            });
        }
    }
}

/// Stream complete lines of a JSONL file starting at `start`, calling `on_record` for JSON
/// objects on lines containing one of `markers` (checked on raw bytes, so huge irrelevant
/// lines are never parsed). Returns `(offset after last complete line, file size)`; a
/// trailing partially-written line is left for the next read.
pub fn read_jsonl_from(
    path: &Path,
    start: u64,
    markers: &[&[u8]],
    on_record: &mut dyn FnMut(&Value, u64),
) -> io::Result<(u64, u64)> {
    let file = File::open(path)?;
    let size = file.metadata()?.len();
    let start = start.min(size);
    let mut reader = BufReader::with_capacity(1 << 20, file);
    reader.seek(SeekFrom::Start(start))?;
    let finders: Vec<memchr::memmem::Finder> =
        markers.iter().map(memchr::memmem::Finder::new).collect();
    let mut pos = start;
    let mut committed = start;
    let mut line = Vec::with_capacity(4096);
    loop {
        line.clear();
        let n = reader.read_until(b'\n', &mut line)?;
        if n == 0 || line.last() != Some(&b'\n') {
            break;
        }
        let line_start = pos;
        pos += n as u64;
        committed = pos;
        let body = &line[..line.len() - 1];
        if body.len() < 2 {
            continue;
        }
        if !finders.is_empty() && !finders.iter().any(|f| f.find(body).is_some()) {
            continue;
        }
        if let Ok(v) = serde_json::from_slice::<Value>(body) {
            if v.is_object() {
                on_record(&v, line_start);
            }
        }
    }
    Ok((committed, size))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cursor {
    pub offset: u64,
    pub size: u64,
    pub mtime: i64,
    pub seen_at: i64,
}

/// Remembers how far each log file has been read so polls only parse new lines. Persisted,
/// so restarting the app never re-reads gigabytes of logs.
#[derive(Debug, Default)]
pub struct FileCursors {
    map: HashMap<String, Cursor>,
}

impl FileCursors {
    /// Read new complete lines. If the file shrank it is re-read from the start (the
    /// ledger's key-based de-duplication prevents double counting).
    pub fn read_jsonl(
        &mut self,
        info: &FileInfo,
        markers: &[&[u8]],
        now: i64,
        on_record: &mut dyn FnMut(&Value, u64),
    ) -> io::Result<bool> {
        let key = info.path.to_string_lossy().to_string();
        let mut start = 0;
        if let Some(cur) = self.map.get_mut(&key) {
            if cur.size == info.size && cur.mtime == info.mtime {
                cur.seen_at = now;
                return Ok(false);
            }
            start = if info.size < cur.offset {
                0
            } else {
                cur.offset
            };
        }
        let (offset, _) = read_jsonl_from(&info.path, start, markers, on_record)?;
        self.map.insert(
            key,
            Cursor {
                offset,
                size: info.size,
                mtime: info.mtime,
                seen_at: now,
            },
        );
        Ok(true)
    }

    /// Whole-file JSON documents: parsed only when the file changed.
    pub fn read_whole_if_changed(&mut self, info: &FileInfo, now: i64) -> Option<Value> {
        let key = info.path.to_string_lossy().to_string();
        if let Some(cur) = self.map.get_mut(&key) {
            if cur.size == info.size && cur.mtime == info.mtime {
                cur.seen_at = now;
                return None;
            }
        }
        let v = fs::read(&info.path)
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok());
        self.map.insert(
            key,
            Cursor {
                offset: info.size,
                size: info.size,
                mtime: info.mtime,
                seen_at: now,
            },
        );
        v
    }

    pub fn prune(&mut self, now: i64, max_age_ms: i64) {
        self.map.retain(|_, c| c.seen_at >= now - max_age_ms);
    }

    pub fn to_vec(&self) -> Vec<(String, Cursor)> {
        self.map
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub fn load(&mut self, entries: Vec<(String, Cursor)>) {
        self.map = entries.into_iter().collect();
    }
}

/// Split a user-entered list of paths ("a, b\nc"), expanding `~`.
pub fn split_paths(value: &str, home: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for raw in value.split(['\n', ',', ';']) {
        let s = raw.trim();
        if s.is_empty() {
            continue;
        }
        let p = if s == "~" {
            home.to_path_buf()
        } else if let Some(rest) = s.strip_prefix("~/").or_else(|| s.strip_prefix("~\\")) {
            home.join(rest)
        } else {
            PathBuf::from(s)
        };
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out
}

pub fn dedup_paths(list: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for p in list {
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn streams_lines_and_keeps_partial_tail() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("a.jsonl");
        let mut f = File::create(&p).unwrap();
        let mut expected = Vec::new();
        let mut full = String::new();
        for i in 0..300 {
            let hit = i % 3 == 0;
            let line = format!(
                "{{\"i\":{i},\"pad\":\"{}\"{}}}\n",
                "x".repeat(i * 7),
                if hit { ",\"hit\":true" } else { "" }
            );
            full.push_str(&line);
            if hit {
                expected.push(i);
            }
        }
        f.write_all(full.as_bytes()).unwrap();
        f.write_all(b"{\"i\":999,\"hit\":true").unwrap();
        drop(f);
        let mut seen = Vec::new();
        let (off, _) = read_jsonl_from(&p, 0, &[b"\"hit\""], &mut |v, _| {
            seen.push(v["i"].as_u64().unwrap() as usize)
        })
        .unwrap();
        assert_eq!(seen, expected);
        assert_eq!(off, full.len() as u64);
    }

    #[test]
    fn splits_paths() {
        let home = Path::new("/home/me");
        let v = split_paths("~/.claude, /x/y\n/x/y;~", home);
        assert_eq!(
            v,
            vec![
                PathBuf::from("/home/me/.claude"),
                PathBuf::from("/x/y"),
                PathBuf::from("/home/me")
            ]
        );
    }
}
