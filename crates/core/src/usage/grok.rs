//! xAI Grok (Grok Build, community grok-cli, or anything logging OpenAI-style `usage`).
//!
//! xAI publishes no usage endpoint for subscription plans, so this provider scans
//! `~/.grok` (plus extra folders) for `*.jsonl` / `*.json` logs with OpenAI-compatible
//! `usage` objects; limits come from the token budgets in Settings (estimated windows) or
//! from a custom command.

use super::common::*;
use super::files::{dedup_paths, list_files, split_paths, ListOptions};
use super::ledger::{Ledger, Tokens};
use crate::time::{parse_time, DAY};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Default)]
pub struct Grok;

impl Grok {
    fn dirs(ctx: &Ctx) -> Vec<PathBuf> {
        let mut dirs = vec![ctx.env.home.join(".grok")];
        dirs.extend(split_paths(&ctx.cfg.extra_paths, &ctx.env.home));
        dedup_paths(dirs)
    }
}

impl Provider for Grok {
    fn id(&self) -> &'static str {
        "grok"
    }
    fn name(&self) -> &'static str {
        "Grok"
    }

    fn scan(&mut self, ctx: &mut Ctx, now: i64) -> usize {
        let since = now - ctx.lookback_days * DAY;
        let mut files = 0;
        let matches = |n: &str| {
            let l = n.to_lowercase();
            l.ends_with(".jsonl")
                || (l.ends_with(".json")
                    && !["settings", "config", "creds", "auth"]
                        .iter()
                        .any(|s| l.contains(s)))
        };
        for dir in Self::dirs(ctx) {
            let list = list_files(
                &dir,
                &ListOptions {
                    matches: &matches,
                    skip_dir: None,
                    modified_since: since,
                    max_depth: 5,
                },
            );
            for f in list {
                files += 1;
                let ns = f
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let mtime = f.mtime;
                if ns.ends_with(".jsonl") {
                    let ledger = &mut *ctx.ledger;
                    let _ = ctx
                        .cursors
                        .read_jsonl(&f, &[b"_tokens\""], now, &mut |rec, offset| {
                            let base_ts =
                                rec.get("timestamp").and_then(parse_time).unwrap_or(mtime);
                            let mut i = 0;
                            walk_usage(rec, 0, None, &mut |u, holder, ts| {
                                let id = holder
                                    .and_then(|h| {
                                        str_of(h.get("id")).or_else(|| str_of(h.get("response_id")))
                                    })
                                    .map(String::from);
                                let key = format!(
                                    "{ns}:{}",
                                    id.unwrap_or_else(|| format!("{offset}#{i}"))
                                );
                                i += 1;
                                record(ledger, &key, u, holder, ts.unwrap_or(base_ts), now);
                            });
                        });
                } else if f.size < 64 * 1024 * 1024 {
                    if let Some(doc) = ctx.cursors.read_whole_if_changed(&f, now) {
                        let mut i = 0;
                        let ledger = &mut *ctx.ledger;
                        walk_usage(&doc, 0, None, &mut |u, holder, ts| {
                            record(
                                ledger,
                                &format!("{ns}#{i}"),
                                u,
                                holder,
                                ts.unwrap_or(mtime),
                                now,
                            );
                            i += 1;
                        });
                    }
                }
            }
        }
        files
    }

    fn windows(&self, _st: &ProviderState, _now: i64) -> Vec<UsageWindow> {
        vec![]
    }
}

fn record(ledger: &mut Ledger, key: &str, u: &Value, holder: Option<&Value>, ts: i64, now: i64) {
    let cached = [
        num(u.get("cached_tokens")),
        num(u.pointer("/prompt_tokens_details/cached_tokens")),
        num(u.get("cache_read_input_tokens")),
    ]
    .into_iter()
    .fold(0.0, f64::max);
    let input = num(u.get("prompt_tokens")).max(num(u.get("input_tokens")));
    let mut output = num(u.get("completion_tokens")).max(num(u.get("output_tokens")));
    if u.get("completion_tokens_details").is_none() {
        output += num(u.get("reasoning_tokens"));
    }
    if input <= 0.0 && output <= 0.0 {
        return;
    }
    let t = Tokens {
        input: (input - cached).max(0.0),
        cache_read: cached,
        output,
        cache_write: 0.0,
    };
    ledger.record(
        "grok",
        key,
        ts,
        t,
        holder.and_then(|h| str_of(h.get("model"))),
        now,
    );
}

/// Find OpenAI-style usage objects anywhere in a record (bounded depth).
pub fn walk_usage<'a>(
    node: &'a Value,
    depth: usize,
    ts: Option<i64>,
    cb: &mut dyn FnMut(&'a Value, Option<&'a Value>, Option<i64>),
) {
    if depth > 6 {
        return;
    }
    match node {
        Value::Array(a) => a.iter().for_each(|v| walk_usage(v, depth + 1, ts, cb)),
        Value::Object(o) => {
            let local_ts = ["timestamp", "created_at", "createdAt", "created"]
                .iter()
                .find_map(|k| o.get(*k).and_then(parse_time))
                .or(ts);
            if let Some(u) = o.get("usage").filter(|u| u.is_object()) {
                if [
                    "prompt_tokens",
                    "input_tokens",
                    "completion_tokens",
                    "output_tokens",
                ]
                .iter()
                .any(|k| u.get(*k).is_some())
                {
                    cb(u, Some(node), local_ts);
                    return;
                }
            }
            for (k, v) in o {
                if k != "usage" && (v.is_object() || v.is_array()) {
                    walk_usage(v, depth + 1, local_ts, cb);
                }
            }
        }
        _ => {}
    }
}
