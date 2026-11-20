//! Self-update from GitHub Releases (signed by CI, see .github/workflows/release.yml).
//! Release builds check shortly after start and every few hours; when a newer version is
//! published they notify, download, install and restart on their own. Dev and local builds
//! only check when asked, so `cargo tauri dev` is never replaced by the released installer.

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

/// Set by the release workflow; builds made anywhere else don't update themselves.
pub const RELEASE_BUILD: bool = option_env!("AGENTMETER_RELEASE").is_some();

static STATUS: Mutex<Option<Value>> = Mutex::new(None);
static BUSY: AtomicBool = AtomicBool::new(false);

/// Last known update state for the UI:
/// idle | checking | latest | available | downloading | installing | error.
pub fn status() -> Value {
    let mut v = STATUS
        .lock()
        .ok()
        .and_then(|s| s.clone())
        .unwrap_or_else(|| json!({ "state": "idle" }));
    v["auto"] = json!(RELEASE_BUILD);
    v
}

fn set(app: &AppHandle, mut v: Value) {
    v["auto"] = json!(RELEASE_BUILD);
    if let Ok(mut s) = STATUS.lock() {
        *s = Some(v.clone());
    }
    let _ = app.emit("event", json!({ "type": "update", "update": v }));
}

pub fn spawn(app: AppHandle) {
    if !RELEASE_BUILD {
        return;
    }
    std::thread::spawn(move || {
        std::thread::sleep(FIRST_CHECK);
        loop {
            if app.state::<AppState>().settings().auto_update {
                run(&app, true);
            }
            std::thread::sleep(EVERY);
        }
    });
}

/// "업데이트 확인": installs right away only when automatic updates are on;
/// otherwise it reports the new version and waits for [`install_now`].
pub fn check_now(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let install = RELEASE_BUILD && app.state::<AppState>().settings().auto_update;
        run(&app, install)
    });
}

/// "설치하고 다시 시작".
pub fn install_now(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || run(&app, true));
}

fn run(app: &AppHandle, install: bool) {
    if BUSY.swap(true, Ordering::SeqCst) {
        return;
    }
    let a = app.clone();
    if let Err(e) = tauri::async_runtime::block_on(async move { check(&a, install).await }) {
        set(app, json!({ "state": "error", "error": e, "at": now_ms() }));
    }
    BUSY.store(false, Ordering::SeqCst);
}

async fn check(app: &AppHandle, install: bool) -> Result<(), String> {
    set(app, json!({ "state": "checking" }));
    let flush_app = app.clone();
    let update = app
        .updater_builder()
        // Windows runs the installer and exits the process right after this
        .on_before_exit(move || crate::flush(&flush_app))
        .build()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;
    let Some(update) = update else {
        set(app, json!({ "state": "latest", "at": now_ms() }));
        return Ok(());
    };
    let version = update.version.clone();
    if !install {
        set(
            app,
            json!({ "state": "available", "version": version, "at": now_ms() }),
        );
        return Ok(());
    }
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
    // On failure nothing has been shut down yet, so the app keeps working.
    update.install(bytes).map_err(|e| e.to_string())?;
    crate::flush(app);
    app.restart();
}
