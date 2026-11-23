//! Claude Code
//! * Logs:   `<config>/projects/<project>/<session>.jsonl` — assistant messages carry `message.usage`.
//! * Limits: `GET https://api.anthropic.com/api/oauth/usage` with the OAuth token Claude Code
//!   stores in `<config>/.credentials.json` (or the macOS Keychain). Read-only: the token is
//!   never refreshed or written here — Claude Code does that itself when it runs.

use super::common::*;
use super::files::{dedup_paths, list_files, split_paths, ListOptions};
use super::ledger::Tokens;
use crate::time::{parse_time, DAY};
use serde_json::Value;
use std::path::PathBuf;

pub const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
/// Account profile: the current plan. `.credentials.json` keeps the plan from the last
/// login, so an upgrade (Pro -> Max) doesn't show there until the user logs in again.
pub const PROFILE_URL: &str = "https://api.anthropic.com/api/oauth/profile";
const PLAN_EVERY_MS: i64 = 30 * 60 * 1000;

/// Plan from a rate-limit tier such as `default_claude_max_5x` or `..._ultra`.
/// Today's plans are Max (5x Pro) and Ultra; `max_20x` is the older Max 20x.
fn tier_label(tier: &str) -> Option<String> {
    let t = tier.to_ascii_lowercase();
    if t.contains("ultra") {
        return Some("Ultra".into());
    }
    if t.contains("max") {
        return Some(if t.contains("20x") { "Max 20x" } else { "Max" }.into());
    }
    None
}

/// `claude_max` -> "Max", `ultra` -> "Ultra"; a plan this app doesn't know yet still
/// shows, with its first letter capitalized.
fn plan_name(kind: &str) -> Option<String> {
    let k = kind.trim().to_ascii_lowercase();
    let k = k.trim_start_matches("claude_");
    if k.is_empty() {
        return None;
    }
    Some(
        k.split('_')
            .filter(|w| !w.is_empty())
            .map(|w| {
                let mut c = w.chars();
                c.next()
                    .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// Plan from the login file: `subscriptionType` plus `rateLimitTier` for the Max size.
pub fn plan_from_credentials(creds: &Value) -> Option<String> {
    str_of(creds.get("rateLimitTier"))
        .and_then(tier_label)
        .or_else(|| str_of(creds.get("subscriptionType")).and_then(plan_name))
}

/// Plan from `/api/oauth/profile`.
pub fn plan_from_profile(v: &Value) -> Option<String> {
    let org = v.get("organization");
    org.and_then(|o| str_of(o.get("rate_limit_tier")))
        .and_then(tier_label)
        .or_else(|| {
            org.and_then(|o| str_of(o.get("organization_type")))
                .and_then(plan_name)
        })
        .or_else(|| {
            let a = v.get("account")?;
            if a.get("has_claude_ultra").and_then(|b| b.as_bool()) == Some(true) {
                Some("Ultra".into())
            } else if a.get("has_claude_max").and_then(|b| b.as_bool()) == Some(true) {
                Some("Max".into())
            } else if a.get("has_claude_pro").and_then(|b| b.as_bool()) == Some(true) {
                Some("Pro".into())
            } else {
                None
            }
        })
}

#[derive(Default)]
pub struct Claude;

impl Claude {
    fn config_dirs(ctx: &Ctx) -> Vec<PathBuf> {
        let mut dirs = vec![];
        if let Some(v) = ctx.env.var("CLAUDE_CONFIG_DIR") {
            dirs.extend(split_paths(v, &ctx.env.home));
        }
        dirs.push(ctx.env.home.join(".config").join("claude"));
        dirs.push(ctx.env.home.join(".claude"));
        dirs.extend(split_paths(&ctx.cfg.extra_paths, &ctx.env.home));
        dedup_paths(dirs)
    }

    fn read_credentials(ctx: &Ctx) -> Option<Value> {
        for dir in Self::config_dirs(ctx) {
            if let Ok(bytes) = std::fs::read(dir.join(".credentials.json")) {
                if let Ok(v) = serde_json::from_slice::<Value>(&bytes) {
                    if str_of(v.pointer("/claudeAiOauth/accessToken")).is_some() {
                        return v.get("claudeAiOauth").cloned();
                    }
                }
            }
        }
        #[cfg(target_os = "macos")]
        {
            let out = std::process::Command::new("security")
                .args([
                    "find-generic-password",
                    "-s",
                    "Claude Code-credentials",
                    "-w",
                ])
                .output()
                .ok()?;
            if out.status.success() {
                let v: Value = serde_json::from_slice(&out.stdout).ok()?;
                if str_of(v.pointer("/claudeAiOauth/accessToken")).is_some() {
                    return v.get("claudeAiOauth").cloned();
                }
            }
        }
        None
    }
}

impl Provider for Claude {
    fn id(&self) -> &'static str {
        "claude"
    }
    fn name(&self) -> &'static str {
        "Claude"
    }
    fn has_api(&self) -> bool {
        true
    }

    fn scan(&mut self, ctx: &mut Ctx, now: i64) -> usize {
        let since = now - ctx.lookback_days * DAY;
        let mut files = 0;
        for dir in Self::config_dirs(ctx) {
            let list = list_files(
                &dir.join("projects"),
                &ListOptions {
                    matches: &|n| n.ends_with(".jsonl"),
                    skip_dir: None,
                    modified_since: since,
                    max_depth: 4,
                },
            );
            for f in list {
                files += 1;
                let ledger = &mut *ctx.ledger;
                let _ = ctx
                    .cursors
                    .read_jsonl(&f, &[b"\"usage\""], now, &mut |rec, _| {
                        on_record(ledger, rec, now)
                    });
            }
        }
        files
    }

    fn poll(&mut self, ctx: &mut Ctx, now: i64) {
        let Some(creds) = Self::read_credentials(ctx) else {
            ctx.state.api_error = Some("Claude Code 로그인 정보를 찾을 수 없습니다".into());
            return;
        };
        // the account profile (below) wins; the login file is only a starting point
        if ctx.state.plan_checked_at == 0 || ctx.state.plan.is_none() {
            if let Some(plan) = plan_from_credentials(&creds) {
                ctx.state.plan = Some(plan);
            }
        }
        ctx.state.auth_expires_at = creds
            .get("expiresAt")
            .and_then(|v| v.as_f64())
            .map(|e| e as i64);
        if let Some(exp) = creds.get("expiresAt").and_then(|v| v.as_f64()) {
            if (exp as i64) < now {
                ctx.state.api_error =
                    Some("토큰 만료 — Claude Code를 한 번 실행하면 갱신됩니다".into());
                return;
            }
        }
        let token = str_of(creds.get("accessToken"))
            .unwrap_or_default()
            .to_string();
        let headers = [
            ("Authorization", format!("Bearer {token}")),
            ("anthropic-beta", "oauth-2025-04-20".to_string()),
            ("Content-Type", "application/json".to_string()),
            ("User-Agent", "agentmeter/0.1".to_string()),
        ];
        if now - ctx.state.plan_checked_at >= PLAN_EVERY_MS {
            if let Ok(r) = fetch_json(ctx.http, "GET", PROFILE_URL, &headers, None) {
                if let (true, Some(j)) = (r.ok(), r.json.as_ref()) {
                    ctx.state.plan_checked_at = now;
                    if let Some(plan) = plan_from_profile(j) {
                        ctx.state.plan = Some(plan);
                    }
                }
            }
        }
        match fetch_json(ctx.http, "GET", USAGE_URL, &headers, None) {
            Err(e) => {
                ctx.state.api_error = Some(format!(
                    "연결 실패: {}",
                    e.chars().take(120).collect::<String>()
                ))
            }
            Ok(r) if r.status == 429 => {
                ctx.state.api_error = Some("요청이 너무 많습니다 (잠시 후 재시도)".into());
                ctx.backoff = true;
            }
            Ok(r) => match (r.ok(), r.json) {
                (true, Some(j)) => {
                    ctx.state.api_error = None;
                    let windows = parse_usage(&j);
                    for w in &windows {
                        ctx.state.record_sample(&w.id, now, w.used_percent);
                    }
                    ctx.state.limits = Some(Limits { at: now, windows });
                }
                _ => ctx.state.api_error = Some(format!("사용량 API 오류 ({})", r.status)),
            },
        }
    }
}

fn on_record(ledger: &mut super::ledger::Ledger, rec: &Value, now: i64) {
    let Some(msg) = rec.get("message").filter(|m| m.is_object()) else {
        return;
    };
    let Some(u) = msg.get("usage").filter(|u| u.is_object()) else {
        return;
    };
    if str_of(msg.get("model")) == Some("<synthetic>") {
        return;
    }
    let Some(ts) = rec.get("timestamp").and_then(parse_time) else {
        return;
    };
    let key = match (str_of(msg.get("id")), str_of(rec.get("requestId"))) {
        (Some(id), Some(req)) => format!("{id}:{req}"),
        (Some(id), None) => id.to_string(),
        _ => match str_of(rec.get("uuid")) {
            Some(u) => u.to_string(),
            None => return,
        },
    };
    let t = Tokens {
        input: num(u.get("input_tokens")),
        output: num(u.get("output_tokens")),
        cache_write: num(u.get("cache_creation_input_tokens")),
        cache_read: num(u.get("cache_read_input_tokens")),
    };
    let project = str_of(rec.get("cwd")).and_then(project_name);
    ledger.record(
        "claude",
        &key,
        ts,
        t,
        str_of(msg.get("model")),
        project.as_deref(),
        now,
    );
}

/// "fable" -> "Fable", "opus_4" -> "Opus 4".
fn model_label(key: &str) -> String {
    key.split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A per-model window ("Fable" -> id `7d_fable`), unless one with that id is already in.
fn push_model_window(
    out: &mut Vec<UsageWindow>,
    name: &str,
    pct: f64,
    resets: Option<i64>,
    minutes: i64,
) {
    let slug: String = name
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    if slug.is_empty() {
        return;
    }
    let short = if minutes == 300 { "5h" } else { "7d" };
    let id = format!("{short}_{slug}");
    if out.iter().any(|w| w.id == id) {
        return;
    }
    let label = if minutes == 300 {
        format!("{} 5H", name.trim())
    } else {
        name.trim().to_string()
    };
    let mut w = UsageWindow::new(&id, &label, pct, resets, "api");
    // Fable has its own switch (on by default); other models are "model-specific"
    w.minor = !slug.contains("fable");
    w.window_minutes = Some(minutes);
    out.push(w);
}

pub fn parse_usage(json: &Value) -> Vec<UsageWindow> {
    let map = [
        ("five_hour", "5h", "5H", false),
        ("seven_day", "7d", "WK", false),
        ("seven_day_opus", "7d_opus", "Opus", true),
        ("seven_day_sonnet", "7d_sonnet", "Sonnet", true),
    ];
    let mut out = vec![];
    for (field, id, label, minor) in map {
        let Some(p) = json.get(field).filter(|p| p.is_object()) else {
            continue;
        };
        let Some(pct) = p
            .get("utilization")
            .and_then(|v| v.as_f64())
            .and_then(clamp_pct)
        else {
            continue;
        };
        let mut w = UsageWindow::new(
            id,
            label,
            pct,
            p.get("resets_at").and_then(parse_time),
            "api",
        );
        w.minor = minor;
        w.window_minutes = Some(if id == "5h" { 300 } else { 10080 });
        out.push(w);
    }
    // Other per-model limits (e.g. `seven_day_fable`), whatever the API adds next.
    if let Some(obj) = json.as_object() {
        let mut extra: Vec<(&String, &Value)> = obj
            .iter()
            .filter(|(k, _)| !map.iter().any(|(f, ..)| f == k))
            .collect();
        extra.sort_by_key(|(k, _)| k.as_str());
        for (key, p) in extra {
            let (prefix, minutes, short) = if let Some(m) = key.strip_prefix("seven_day_") {
                (m, 10080, "7d")
            } else if let Some(m) = key.strip_prefix("five_hour_") {
                (m, 300, "5h")
            } else {
                continue;
            };
            if prefix.is_empty()
                || prefix == "oauth_apps"
                || prefix.contains("overage")
                || prefix.contains("breakdown")
                || prefix.ends_with("dollars")
                || !p.is_object()
            {
                continue;
            }
            let Some(pct) = p
                .get("utilization")
                .and_then(|v| v.as_f64())
                .and_then(clamp_pct)
            else {
                continue;
            };
            let name = model_label(prefix);
            let label = if minutes == 300 {
                format!("{name} 5H")
            } else {
                name
            };
            let mut w = UsageWindow::new(
                &format!("{short}_{prefix}"),
                &label,
                pct,
                p.get("resets_at").and_then(parse_time),
                "api",
            );
            // Fable has its own switch (on by default); other models are "model-specific".
            w.minor = !prefix.contains("fable");
            w.window_minutes = Some(minutes);
            out.push(w);
        }
    }
    // Newer responses list every limit in `limits[]`; per-model weekly caps (Fable …)
    // appear only there, while `seven_day_<model>` stays null.
    //   { "kind": "weekly_scoped", "percent": 43, "resets_at": "…",
    //     "scope": { "model": { "id": null, "display_name": "Fable" } }, "is_active": true }
    if let Some(list) = json.get("limits").and_then(|v| v.as_array()) {
        for l in list {
            let kind = str_of(l.get("kind")).unwrap_or("");
            let group = str_of(l.get("group")).unwrap_or("");
            let Some(pct) = l
                .get("percent")
                .or_else(|| l.get("utilization"))
                .and_then(|v| v.as_f64())
                .and_then(clamp_pct)
            else {
                continue;
            };
            let resets = l.get("resets_at").and_then(parse_time);
            let session = kind.contains("session") || group == "session";
            let model = str_of(l.pointer("/scope/model/display_name"))
                .or_else(|| str_of(l.pointer("/scope/model/id")));
            match model {
                Some(name) => push_model_window(
                    &mut out,
                    name,
                    pct,
                    resets,
                    if session { 300 } else { 10080 },
                ),
                // the all-model windows only fill in when the classic fields are missing
                None if kind == "session" && !out.iter().any(|w| w.id == "5h") => {
                    let mut w = UsageWindow::new("5h", "5H", pct, resets, "api");
                    w.window_minutes = Some(300);
                    out.insert(0, w);
                }
                None if kind == "weekly_all" && !out.iter().any(|w| w.id == "7d") => {
                    let mut w = UsageWindow::new("7d", "WK", pct, resets, "api");
                    w.window_minutes = Some(10080);
                    let at = out.iter().position(|w| w.id != "5h").unwrap_or(out.len());
                    out.insert(at, w);
                }
                None => {}
            }
        }
    }
    // the same per-model numbers as a separate list in some responses
    if let Some(list) = json.get("model_scoped").and_then(|v| v.as_array()) {
        for m in list {
            let (Some(name), Some(pct)) = (
                str_of(m.get("display_name")),
                m.get("utilization")
                    .or_else(|| m.get("percent"))
                    .and_then(|v| v.as_f64())
                    .and_then(clamp_pct),
            ) else {
                continue;
            };
            push_model_window(
                &mut out,
                name,
                pct,
                m.get("resets_at").and_then(parse_time),
                10080,
            );
        }
    }
    if let Some(e) = json.get("extra_usage") {
        if e.get("is_enabled").and_then(|v| v.as_bool()) == Some(true) {
            if let Some(pct) = e
                .get("utilization")
                .and_then(|v| v.as_f64())
                .and_then(clamp_pct)
            {
                let mut w = UsageWindow::new("extra", "Extra", pct, None, "api");
                w.minor = true;
                out.push(w);
            }
        }
    }
    out
}
