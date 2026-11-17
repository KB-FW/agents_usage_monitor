//! Background threads: usage polling, gift timers, CPU/RAM sampling and the pet window's
//! click-through hit testing.

use crate::http::UreqHttp;
use crate::state::{emit_event, lock, push_game, AppState, UsageCmd};
use agentpets_core::catalog::catalog;
use agentpets_core::time::now_ms;
use agentpets_core::usage::{Env, Snapshot, UsageService};
use serde_json::json;
use std::collections::BTreeMap;
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
    let credit_since = lock(&st.game).installed_at;
    thread::Builder::new()
        .name("usage".into())
        .spawn(move || {
            let mut svc = UsageService::new(
                Some(cache),
                Env::from_system(),
                Box::new(UreqHttp::new()),
                credit_since,
            );
            svc.load();
            let mut force = false;
            loop {
                let st = app.state::<AppState>();
                let settings = st.settings();
                let r = svc.refresh(&settings, force, now_ms());
                on_usage(&app, r.snapshot);
                if !r.credits.is_empty() {
                    on_credit(&app, &r.credits);
                }
                force = false;
                match rx.recv_timeout(Duration::from_secs(settings.poll_logs_sec.max(5))) {
                    Ok(UsageCmd::Refresh { force: f }) => force = f,
                    Ok(UsageCmd::SetCreditSince(ts)) => svc.set_credit_since(ts),
                    Ok(UsageCmd::Stop(ack)) => {
                        svc.save(now_ms(), true);
                        let _ = ack.send(());
                        break;
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => {
                        svc.save(now_ms(), true);
                        break;
                    }
                }
            }
        })
        .expect("spawn usage thread");
}

fn on_credit(app: &AppHandle, credits: &BTreeMap<String, i64>) {
    let st = app.state::<AppState>();
    let events = lock(&st.game).credit_tokens(credits, now_ms());
    push_game(app);
    for ev in events {
        if ev["type"] == "levelup" {
            let name = catalog()
                .pet(ev["pet"].as_str().unwrap_or(""))
                .map(|p| p.name.clone())
                .unwrap_or_default();
            notify(
                app,
                &format!("{name} 레벨 업!"),
                &format!("Lv.{} 달성 · 보상 ✦{}", ev["level"], ev["sparks"]),
            );
        }
        emit_event(app, ev);
    }
}

fn on_usage(app: &AppHandle, snap: Snapshot) {
    let st = app.state::<AppState>();
    check_alerts(app, &snap);
    let _ = app.emit("usage", &snap);
    *lock(&st.last_usage) = Some(snap);
    crate::menu::refresh_tray(app);
}

/// Threshold alerts (80/95 %) and reset bonuses, compared with the previous snapshot.
fn check_alerts(app: &AppHandle, snap: &Snapshot) {
    let st = app.state::<AppState>();
    let settings = st.settings();
    let thresholds = settings.alert_thresholds.clone();
    let now = now_ms();
    let mut bonus_added = false;
    for p in snap.providers.iter().filter(|p| p.enabled) {
        let pet = catalog()
            .pet_for_provider(&p.id)
            .map(|x| x.id.clone())
            .unwrap_or_default();
        for w in p.windows.iter().filter(|w| !w.minor) {
            let key = format!("{}:{}", p.id, w.id);
            let level = thresholds.iter().filter(|t| w.used_percent >= **t).count();
            let prev = lock(&st.alerts).insert(key, (w.used_percent, level));
            let Some((prev_pct, prev_level)) = prev else {
                continue;
            }; // no alerts on start-up
            if level > prev_level {
                emit_event(
                    app,
                    json!({ "type": "alert", "provider": p.id, "pet": pet, "label": w.label, "percent": w.used_percent, "threshold": thresholds[level - 1] }),
                );
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
            if prev_pct >= 50.0 && w.used_percent <= prev_pct - 40.0 {
                let added = {
                    let mut g = lock(&st.game);
                    let mut rng = lock(&st.rng);
                    g.add_bonus_bubble(
                        &pet,
                        now,
                        &format!("{} {} 리셋", p.name, w.label),
                        &mut *rng,
                    )
                };
                bonus_added |= added;
                emit_event(
                    app,
                    json!({ "type": "reset", "provider": p.id, "pet": pet, "label": w.label }),
                );
                notify(
                    app,
                    &format!("{} {} 사용량 리셋!", p.name, w.label),
                    "보너스 선물이 도착했어요 🎁",
                );
            }
        }
    }
    if bonus_added {
        push_game(app);
    }
}

// ------------------------------------------------------------------ gift timers
pub fn spawn_ticker(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(20));
        let st = app.state::<AppState>();
        let spawned = {
            let mut g = lock(&st.game);
            let mut rng = lock(&st.rng);
            g.tick(now_ms(), &mut *rng)
        };
        if spawned > 0 {
            push_game(&app);
        }
    });
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
            if app.get_webview_window("bar").is_some() {
                let _ = app.emit_to("bar", "system", v);
            }
            thread::sleep(Duration::from_secs(2));
        }
    });
}

// ------------------------------------------------------------------ click-through
/// The pet window ignores the mouse except over the rectangles reported by the UI, so the
/// transparent parts of the window never block clicks on the desktop.
pub fn spawn_hit_test(app: AppHandle) {
    thread::spawn(move || {
        let mut ignoring: Option<bool> = None;
        loop {
            thread::sleep(Duration::from_millis(45));
            let Some(win) = app.get_webview_window("pet") else {
                ignoring = None;
                continue;
            };
            let st = app.state::<AppState>();
            let want_ignore = if !lock(&st.settings).click_through || lock(&st.drag).is_some() {
                false
            } else {
                let (Ok(cursor), Ok(inner), Ok(sf)) = (
                    win.cursor_position(),
                    win.inner_position(),
                    win.scale_factor(),
                ) else {
                    continue;
                };
                let lx = (cursor.x - inner.x as f64) / sf;
                let ly = (cursor.y - inner.y as f64) / sf;
                let inside = lock(&st.hit_regions)
                    .iter()
                    .any(|r| lx >= r[0] && lx <= r[0] + r[2] && ly >= r[1] && ly <= r[1] + r[3]);
                !inside
            };
            if ignoring != Some(want_ignore) && win.set_ignore_cursor_events(want_ignore).is_ok() {
                ignoring = Some(want_ignore);
            }
        }
    });
}
