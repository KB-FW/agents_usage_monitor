//! OpenAI Codex CLI
//! * Rollouts: `$CODEX_HOME/sessions/YYYY/MM/DD/rollout-*.jsonl`
//! * `event_msg` records with `payload.type == "token_count"` carry
//!   `info.total_token_usage` / `info.last_token_usage` and
//!   `rate_limits.primary|secondary { used_percent, window_minutes, resets_at }`
//!   (primary = 5-hour window, secondary = weekly window on paid plans).
//! * Codex also runs sessions on its own (memory consolidation when the app starts, guardian
//!   command reviews). Their tokens count, but they are not user activity and their limit
//!   snapshots are not used.

use super::common::*;
use super::files::{dedup_paths, list_files, split_paths, ListOptions};
use super::ledger::{Ledger, Tokens};
use crate::time::{parse_time, DAY};
use chrono::{Datelike, Local, TimeZone};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// What we know about a rollout file from its context records.
#[derive(Default)]
struct FileCtx {
    model: Option<String>,
    project: Option<String>,
    /// A session Codex started by itself.
    background: bool,
}

impl FileCtx {
    fn apply(&mut self, payload: &Value) {
        if let Some(m) = str_of(payload.get("model")) {
            self.model = Some(m.to_string());
        }
        if let Some(p) = str_of(payload.get("cwd")).and_then(project_name) {
            self.project = Some(p);
        }
        if payload.get("source").is_some() || payload.get("thread_source").is_some() {
            self.background = is_background(payload);
        }
    }
}

/// `session_meta.source` is `"cli"`, `"vscode"`, … or `{ "internal": "memory_consolidation" }`;
/// newer builds also write `thread_source`.
fn is_background(meta: &Value) -> bool {
    let internal = match meta.get("source") {
        Some(Value::Object(m)) => m.keys().any(|k| k.eq_ignore_ascii_case("internal")),
        Some(Value::String(s)) => s.to_ascii_lowercase().starts_with("internal"),
        _ => false,
    };
    internal
        || matches!(
            str_of(meta.get("thread_source")),
            Some("memory_consolidation" | "guardian_review")
        )
}

/// The first line of a rollout is its `session_meta`; read it when we resume a file mid-way.
fn read_meta(path: &Path) -> Option<FileCtx> {
    use std::io::{BufRead, Read};
    let f = std::fs::File::open(path).ok()?;
    let mut line = String::new();
    std::io::BufReader::new(f.take(4 << 20))
        .read_line(&mut line)
        .ok()?;
    let rec: Value = serde_json::from_str(line.trim()).ok()?;
    if str_of(rec.get("type")) != Some("session_meta") {
        return None;
    }
    let mut fc = FileCtx::default();
    fc.apply(rec.get("payload")?);
    Some(fc)
}

#[derive(Default)]
pub struct Codex {
    files: HashMap<PathBuf, FileCtx>,
}

impl Codex {
    fn homes(ctx: &Ctx) -> Vec<PathBuf> {
        let mut dirs = vec![];
        if let Some(v) = ctx.env.var("CODEX_HOME") {
            dirs.extend(split_paths(v, &ctx.env.home));
        }
        dirs.push(ctx.env.home.join(".codex"));
        dirs.extend(split_paths(&ctx.cfg.extra_paths, &ctx.env.home));
        dedup_paths(dirs)
    }
}

impl Provider for Codex {
    fn id(&self) -> &'static str {
        "codex"
    }
    fn name(&self) -> &'static str {
        "Codex"
    }

    fn scan(&mut self, ctx: &mut Ctx, now: i64) -> usize {
        let since = now - ctx.lookback_days * DAY;
        let (min_year, min_month) = match Local.timestamp_millis_opt(since - DAY).earliest() {
            Some(d) => (d.year(), format!("{:04}-{:02}", d.year(), d.month())),
            None => (1970, "1970-01".into()),
        };
        let mut files = 0;
        let mut snaps = Vec::new();
        for home in Self::homes(ctx) {
            for sub in ["sessions", "archived_sessions"] {
                let root = home.join(sub);
                let root_c = root.clone();
                let min_month_c = min_month.clone();
                let skip = move |name: &str, depth: usize, full: &Path| -> bool {
                    // prune date folders clearly older than the lookback window
                    let is_num =
                        |s: &str, n: usize| s.len() == n && s.bytes().all(|b| b.is_ascii_digit());
                    if depth == 0 && is_num(name, 4) {
                        return name.parse::<i32>().unwrap_or(0) < min_year;
                    }
                    if depth == 1 && is_num(name, 2) {
                        if let Some(year) = full
                            .strip_prefix(&root_c)
                            .ok()
                            .and_then(|r| r.components().next())
                        {
                            let y = year.as_os_str().to_string_lossy();
                            if is_num(&y, 4) {
                                return format!("{y}-{name}") < min_month_c;
                            }
                        }
                    }
                    false
                };
                let list = list_files(
                    &root,
                    &ListOptions {
                        matches: &|n| n.ends_with(".jsonl"),
                        skip_dir: Some(&skip),
                        modified_since: since,
                        max_depth: 4,
                    },
                );
                for f in list {
                    files += 1;
                    if !self.files.contains_key(&f.path) {
                        if let Some(fc) = read_meta(&f.path) {
                            self.files.insert(f.path.clone(), fc);
                        }
                    }
                    let ledger = &mut *ctx.ledger;
                    let snaps = &mut snaps;
                    let files_ctx = &mut self.files;
                    let path = f.path.clone();
                    let _ = ctx.cursors.read_jsonl(
                        &f,
                        &[b"\"token_count\"", b"\"turn_context\"", b"\"session_meta\""],
                        now,
                        &mut |rec, offset| {
                            on_record(ledger, snaps, files_ctx, &path, rec, offset, now)
                        },
                    );
                }
            }
        }
        apply_snaps(ctx.state, snaps);
        files
    }

    fn windows(&self, st: &ProviderState, _now: i64) -> Vec<UsageWindow> {
        st.limits
            .as_ref()
            .map(|l| l.windows.clone())
            .unwrap_or_default()
    }
}

fn on_record(
    ledger: &mut Ledger,
    snaps: &mut Vec<Snap>,
    files: &mut HashMap<PathBuf, FileCtx>,
    file: &Path,
    rec: &Value,
    offset: u64,
    now: i64,
) {
    let Some(payload) = rec.get("payload").filter(|p| p.is_object()) else {
        return;
    };
    let kind = str_of(rec.get("type"));
    if kind == Some("turn_context") || kind == Some("session_meta") {
        files.entry(file.to_path_buf()).or_default().apply(payload);
        return;
    }
    if kind != Some("event_msg") || str_of(payload.get("type")) != Some("token_count") {
        return;
    }
    let Some(ts) = rec.get("timestamp").and_then(parse_time) else {
        return;
    };
    let background = files.get(file).map(|f| f.background).unwrap_or(false);

    if let Some(last) = payload
        .pointer("/info/last_token_usage")
        .filter(|v| v.is_object())
    {
        let total = payload.pointer("/info/total_token_usage");
        let cached =
            num(last.get("cached_input_tokens")).max(num(last.get("cache_read_input_tokens")));
        // The cumulative total identifies a turn: duplicate emissions and copied history
        // in resumed sessions map to the same key.
        let key = match total
            .and_then(|t| t.get("total_tokens"))
            .and_then(|v| v.as_i64())
        {
            Some(cum) => format!(
                "{cum}:{}:{}",
                last.get("total_tokens")
                    .and_then(|v| v.as_i64())
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                last.get("input_tokens")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0)
            ),
            None => format!(
                "{}@{offset}",
                file.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default()
            ),
        };
        let t = Tokens {
            input: (num(last.get("input_tokens")) - cached).max(0.0),
            cache_read: cached,
            output: num(last.get("output_tokens")),
            cache_write: 0.0,
        };
        let record = if background {
            Ledger::record_background
        } else {
            Ledger::record
        };
        record(
            ledger,
            "codex",
            &key,
            ts,
            t,
            files.get(file).and_then(|f| f.model.as_deref()),
            files.get(file).and_then(|f| f.project.as_deref()),
            now,
        );
    }

    if background {
        return;
    }
    if let Some(rl) = payload.get("rate_limits").filter(|r| r.is_object()) {
        let limit_id = str_of(rl.get("limit_id"));
        if limit_id.is_none() || limit_id == Some("codex") {
            let windows = parse_rate_limits(rl, ts);
            if !windows.is_empty() {
                snaps.push(Snap {
                    ts,
                    windows,
                    plan: str_of(rl.get("plan_type")).map(String::from),
                });
            }
        }
    }
}

/// A limit snapshot found while scanning; applied in time order once all files are read.
struct Snap {
    ts: i64,
    windows: Vec<UsageWindow>,
    plan: Option<String>,
}

fn apply_snaps(state: &mut ProviderState, mut snaps: Vec<Snap>) {
    snaps.sort_by_key(|s| s.ts);
    for snap in snaps {
        if state.limits.as_ref().is_some_and(|l| snap.ts < l.at) {
            continue;
        }
        let windows: Vec<UsageWindow> = match &state.limits {
            Some(prev) => snap
                .windows
                .into_iter()
                .map(|w| keep_consistent(w, &prev.windows, snap.ts))
                .collect(),
            None => snap.windows,
        };
        for w in &windows {
            state.record_sample(&w.id, snap.ts, w.used_percent);
        }
        state.limits = Some(Limits {
            at: snap.ts,
            windows,
        });
        if let Some(plan) = snap.plan {
            state.plan = Some(plan);
        }
    }
}

/// Usage inside a window can't go down before that window resets. A snapshot saying so
/// (0% with a brand-new reset time while the known window is still running) belongs to some
/// other quota, so keep what we had for that window.
fn keep_consistent(new: UsageWindow, prev: &[UsageWindow], ts: i64) -> UsageWindow {
    let Some(old) = prev.iter().find(|o| o.id == new.id) else {
        return new;
    };
    let still_running = old.resets_at.map(|r| ts < r - 60_000).unwrap_or(false);
    let reset_moved = match (old.resets_at, new.resets_at) {
        (Some(o), Some(n)) => n > o + 10 * 60_000,
        _ => false,
    };
    if still_running && reset_moved && new.used_percent + 5.0 < old.used_percent {
        old.clone()
    } else {
        new
    }
}

pub fn parse_rate_limits(rl: &Value, event_ts: i64) -> Vec<UsageWindow> {
    let mut out = vec![];
    for slot in ["primary", "secondary"] {
        let Some(w) = rl.get(slot).filter(|w| w.is_object()) else {
            continue;
        };
        let Some(pct) = w
            .get("used_percent")
            .and_then(|v| v.as_f64())
            .and_then(clamp_pct)
        else {
            continue;
        };
        let mins = w
            .get("window_minutes")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let resets = match (w.get("resets_at"), w.get("resets_in_seconds")) {
            (Some(v), _) if !v.is_null() => parse_time(v),
            (_, Some(v)) if !v.is_null() => v.as_f64().map(|s| event_ts + (s * 1000.0) as i64),
            _ => None,
        };
        let label = if mins > 0 {
            window_label(mins)
        } else if slot == "primary" {
            "5H".into()
        } else {
            "WK".into()
        };
        let id = match mins {
            300 => "5h".to_string(),
            10080 => "7d".to_string(),
            _ => slot.to_string(),
        };
        let mut uw = UsageWindow::new(&id, &label, pct, resets, "log");
        uw.window_minutes = (mins > 0).then_some(mins);
        out.push(uw);
    }
    out
}
