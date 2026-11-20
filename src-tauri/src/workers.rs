//! Background threads: usage polling + alerts, CPU/RAM sampling, and keeping the docked
//! widget on top of the taskbar.

use crate::http::UreqHttp;
use crate::state::{lock, AlertMemo, AppState, UsageCmd};
use crate::windows::{keep_on_top, layout_widget, screen_geometry, Rect, WIDGET};
use agentmeter_core::time::now_ms;
use agentmeter_core::usage::{Env, Snapshot, UsageService};
use serde_json::json;
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

pub fn notify(app: &AppHandle, title: &str, body: &str) {
    if !app.state::<AppState>().settings().notifications {
        return;
    }
    let _ = app.notification().builder().title(title).body(body).show();
}

// ------------------------------------------------------------------ usage
pub fn spawn_usage(app: AppHandle) {
    let (tx, rx) = channel::<UsageCmd>();
    let st = app.state::<AppState>();
    *lock(&st.usage_tx) = Some(tx);
    let cache = st.data_dir.join("usage-cache.json");
    thread::Builder::new()
        .name("usage".into())
        .spawn(move || {
            let mut svc = UsageService::new(
                Some(cache),
                Env::from_system(),
                Box::new(UreqHttp::new()),
                now_ms(),
            );
            svc.load();
            let mut force = false;
            loop {
                let st = app.state::<AppState>();
                let settings = st.settings();
                let r = svc.refresh(&settings, force, now_ms());
                on_usage(&app, r.snapshot);
                force = false;
                // wait for the next poll, answering commands in between
                let deadline =
                    std::time::Instant::now() + Duration::from_secs(settings.poll_logs_sec.max(5));
                loop {
                    let left = deadline.saturating_duration_since(std::time::Instant::now());
                    match rx.recv_timeout(left) {
                        Ok(UsageCmd::Refresh { force: f }) => {
                            force = f;
                            break;
                        }
                        Ok(UsageCmd::ExportCsv(reply)) => {
                            let _ = reply.send(svc.export_csv());
                        }
                        Ok(UsageCmd::Stop(ack)) => {
                            svc.save(now_ms(), true);
                            let _ = ack.send(());
                            return;
                        }
                        Err(RecvTimeoutError::Timeout) => break,
                        Err(RecvTimeoutError::Disconnected) => {
                            svc.save(now_ms(), true);
                            return;
                        }
                    }
                }
            }
        })
        .expect("spawn usage thread");
}

fn on_usage(app: &AppHandle, snap: Snapshot) {
    let st = app.state::<AppState>();
    check_alerts(app, &snap);
    let _ = app.emit("usage", &snap);
    *lock(&st.last_usage) = Some(snap);
    crate::menu::refresh_tray(app);
}

/// Threshold, reset and "runs out before reset" notifications.
fn check_alerts(app: &AppHandle, snap: &Snapshot) {
    let st = app.state::<AppState>();
    let s = st.settings();
    let now = now_ms();
    for p in snap.providers.iter().filter(|p| p.enabled) {
        for w in p.windows.iter().filter(|w| !w.minor) {
            let key = format!("{}:{}", p.id, w.id);
            let level = s
                .alert_thresholds
                .iter()
                .filter(|t| w.used_percent >= **t)
                .count();
            let prev = lock(&st.alerts).get(&key).cloned();
            let mut memo = AlertMemo {
                pct: w.used_percent,
                level,
                forecast_for: prev.as_ref().and_then(|m| m.forecast_for),
            };
            if let Some(prev) = prev {
                // no alerts for the first observation after start-up
                if level > prev.level {
                    let t = s.alert_thresholds[level - 1];
                    let _ = app.emit("event", json!({ "type": "alert", "provider": p.id, "label": w.label, "percent": w.used_percent, "threshold": t }));
                    let body = match w.resets_at {
                        Some(r) => format!("{} 후 리셋", crate::fmt_left(r - now)),
                        None => "사용량을 확인하세요".into(),
                    };
                    notify(
                        app,
                        &format!("{} {} {}% 사용", p.name, w.label, w.used_percent.round()),
                        &body,
                    );
                }
                if s.notify_reset && prev.pct >= 30.0 && w.used_percent <= prev.pct - 30.0 {
                    let _ = app.emit(
                        "event",
                        json!({ "type": "reset", "provider": p.id, "label": w.label }),
                    );
                    notify(
                        app,
                        &format!("{} {} 리셋됨", p.name, w.label),
                        "다시 넉넉하게 쓸 수 있어요",
                    );
                }
            }
            if s.notify_forecast
                && w.exhaust_before_reset
                && w.used_percent >= 40.0
                && memo.forecast_for != w.resets_at
            {
                if let (Some(eta), Some(reset)) = (w.eta, w.resets_at) {
                    memo.forecast_for = w.resets_at;
                    notify(
                        app,
                        &format!("{} {} 리셋 전에 소진 예상", p.name, w.label),
                        &format!(
                            "지금 속도면 {} 후 한도 도달 · 리셋은 {} 후",
                            crate::fmt_left(eta - now),
                            crate::fmt_left(reset - now)
                        ),
                    );
                }
            }
            lock(&st.alerts).insert(key, memo);
        }
    }
}

// ------------------------------------------------------------------ CPU / RAM
pub fn spawn_system(app: AppHandle) {
    thread::spawn(move || {
        let mut sys = sysinfo::System::new();
        loop {
            sys.refresh_cpu_usage();
            sys.refresh_memory();
            let total = sys.total_memory().max(1) as f64;
            let v = json!({ "cpu": sys.global_cpu_usage().round(), "ram": ((sys.used_memory() as f64 / total) * 100.0).round() });
            *lock(&app.state::<AppState>().last_system) = v.clone();
            if app.get_webview_window(WIDGET).is_some() {
                let _ = app.emit_to(WIDGET, "system", v);
            }
            thread::sleep(Duration::from_secs(2));
        }
    });
}

// ------------------------------------------------------------------ taskbar keeper
/// While docked: stay above the taskbar and follow it when the screen layout changes.
pub fn spawn_keeper(app: AppHandle) {
    thread::spawn(move || {
        let mut last_band: Option<Rect> = None;
        loop {
            thread::sleep(Duration::from_millis(1000));
            let Some(win) = app.get_webview_window(WIDGET) else {
                crate::windows::release_top();
                continue;
            };
            let st = app.state::<AppState>();
            let s = st.settings();
            if lock(&st.drag).is_some() {
                continue;
            }
            if s.is_docked() {
                let (_, band, _) = screen_geometry(&app);
                if last_band != Some(band) {
                    if last_band.is_some() {
                        layout_widget(&app, None);
                        let _ = app.emit_to(
                            WIDGET,
                            "event",
                            json!({ "type": "taskbar", "height": band.h }),
                        );
                    }
                    last_band = Some(band);
                }
                keep_on_top(&win);
            } else {
                last_band = None;
                if s.always_on_top {
                    keep_on_top(&win);
                } else {
                    crate::windows::release_top();
                }
            }
        }
    });
}
