//! OpenAI Codex CLI
//! * Rollouts: `$CODEX_HOME/sessions/YYYY/MM/DD/rollout-*.jsonl`
//! * `event_msg` records with `payload.type == "token_count"` carry
//!   `info.total_token_usage` / `info.last_token_usage` and
//!   `rate_limits.primary|secondary { used_percent, window_minutes, resets_at }`
//!   (primary = 5-hour window, secondary = weekly window on paid plans).

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
                    let ledger = &mut *ctx.ledger;
                    let state = &mut *ctx.state;
                    let files_ctx = &mut self.files;
                    let path = f.path.clone();
                    let _ = ctx.cursors.read_jsonl(
                        &f,
                        &[b"\"token_count\"", b"\"turn_context\"", b"\"session_meta\""],
                        now,
                        &mut |rec, offset| {
                            on_record(ledger, state, files_ctx, &path, rec, offset, now)
                        },
                    );
                }
            }
        }
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
    state: &mut ProviderState,
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
        let fc = files.entry(file.to_path_buf()).or_default();
        if let Some(m) = str_of(payload.get("model")) {
            fc.model = Some(m.to_string());
        }
        if let Some(p) = str_of(payload.get("cwd")).and_then(project_name) {
            fc.project = Some(p);
        }
        return;
    }
    if kind != Some("event_msg") || str_of(payload.get("type")) != Some("token_count") {
        return;
    }
    let Some(ts) = rec.get("timestamp").and_then(parse_time) else {
        return;
    };

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
        ledger.record(
            "codex",
            &key,
            ts,
            t,
            files.get(file).and_then(|f| f.model.as_deref()),
            files.get(file).and_then(|f| f.project.as_deref()),
            now,
        );
    }

    if let Some(rl) = payload.get("rate_limits").filter(|r| r.is_object()) {
        let limit_id = str_of(rl.get("limit_id"));
        if limit_id.is_none() || limit_id == Some("codex") {
            let windows = parse_rate_limits(rl, ts);
            for w in &windows {
                state.record_sample(&w.id, ts, w.used_percent);
            }
            if !windows.is_empty() && state.limits.as_ref().map(|l| ts >= l.at).unwrap_or(true) {
                state.limits = Some(Limits { at: ts, windows });
                if let Some(plan) = str_of(rl.get("plan_type")) {
                    state.plan = Some(plan.to_string());
                }
            }
        }
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
