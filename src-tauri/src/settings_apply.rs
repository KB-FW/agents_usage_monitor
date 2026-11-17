//! Apply a settings change: persist, update windows/tray/autostart, broadcast.

use crate::state::{lock, AppState, UsageCmd};
use crate::windows;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

pub fn update_settings(app: &AppHandle, patch: Value) -> Value {
    let st = app.state::<AppState>();
    let (prev, next) = {
        let mut s = lock(&st.settings);
        let prev = s.clone();
        *s = s.merged(&patch);
        (prev, s.clone())
    };
    st.save_settings();
    let has = |k: &str| patch.get(k).is_some();

    if has("showPet") {
        if next.show_pet {
            let _ = windows::create_pet(app);
        } else {
            windows::close(app, "pet");
        }
    }
    if has("petScale") && (prev.pet_scale - next.pet_scale).abs() > f64::EPSILON {
        windows::rescale_pet(app, next.pet_scale);
    }
    if has("showBar") {
        if next.show_bar {
            let _ = windows::create_bar(app);
        } else {
            windows::close(app, "bar");
        }
    }
    if has("alwaysOnTop") {
        if let Some(w) = app.get_webview_window("pet") {
            let _ = w.set_always_on_top(next.always_on_top);
        }
    }
    if has("clickThrough") && !next.click_through {
        if let Some(w) = app.get_webview_window("pet") {
            let _ = w.set_ignore_cursor_events(false);
        }
    }
    if has("launchAtLogin") {
        let al = app.autolaunch();
        let _ = if next.launch_at_login {
            al.enable()
        } else {
            al.disable()
        };
    }
    if has("providers") || has("pollApiSec") {
        st.send_usage(UsageCmd::Refresh { force: true });
    }
    let v = serde_json::to_value(&next).unwrap_or(Value::Null);
    let _ = app.emit("settings", v.clone());
    crate::menu::refresh_tray(app);
    v
}
