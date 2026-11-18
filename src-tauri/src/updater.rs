//! Self-update from GitHub Releases (signed by CI, see .github/workflows/release.yml).
//! Checks shortly after start and every few hours; when a newer version is published it
//! notifies, downloads, installs and restarts on its own.

use crate::state::AppState;
use crate::workers::notify;
use agentmeter_core::time::now_ms;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

const FIRST_CHECK: Duration = Duration::from_secs(20);
const EVERY: Duration = Duration::from_secs(3 * 3600);

static STATUS: Mutex<Option<Value>> = Mutex::new(None);
static BUSY: AtomicBool = AtomicBool::new(false);

/// Last known update state for the UI: idle | checking | latest | downloading | installing | error.
pub fn status() -> Value {
    STATUS
        .lock()
        .ok()
        .and_then(|s| s.clone())
        .unwrap_or_else(|| json!({ "state": "idle" }))
}

fn set(app: &AppHandle, v: Value) {
    if let Ok(mut s) = STATUS.lock() {
        *s = Some(v.clone());
    }
    let _ = app.emit("event", json!({ "type": "update", "update": v }));
}

pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || {
        std::thread::sleep(FIRST_CHECK);
        loop {
            if app.state::<AppState>().settings().auto_update {
                run(&app);
            }
            std::thread::sleep(EVERY);
        }
    });
}

/// Check now (from the dashboard button or the timer).
pub fn check_now(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || run(&app));
}

fn run(app: &AppHandle) {
    if BUSY.swap(true, Ordering::SeqCst) {
        return;
    }
    let a = app.clone();
    if let Err(e) = tauri::async_runtime::block_on(async move { check_and_install(&a).await }) {
        set(app, json!({ "state": "error", "error": e, "at": now_ms() }));
    }
    BUSY.store(false, Ordering::SeqCst);
}

async fn check_and_install(app: &AppHandle) -> Result<(), String> {
    set(app, json!({ "state": "checking" }));
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;
    let Some(update) = update else {
        set(app, json!({ "state": "latest", "at": now_ms() }));
        return Ok(());
    };
    let version = update.version.clone();
    notify(
        app,
        "AgentMeter 업데이트",
        &format!("새 버전 {version}을 설치하고 다시 시작해요."),
    );
    set(
        app,
        json!({ "state": "downloading", "version": version, "progress": 0 }),
    );
    let mut got: u64 = 0;
    let mut shown = 0u64;
    let bytes = update
        .download(
            |chunk, total| {
                got += chunk as u64;
                if let Some(t) = total.filter(|t| *t > 0) {
                    let pct = got * 100 / t;
                    if pct >= shown + 5 {
                        shown = pct;
                        set(
                            app,
                            json!({ "state": "downloading", "version": version, "progress": pct }),
                        );
                    }
                }
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;
    set(app, json!({ "state": "installing", "version": version }));
    crate::flush(app);
    // On Windows this starts the installer and exits; elsewhere we restart ourselves.
    update.install(bytes).map_err(|e| e.to_string())?;
    app.restart();
}
