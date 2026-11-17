//! Commands invoked from the UI (`window.agentMeter` → ui/shared/bridge.js).

use crate::apply::update_settings;
use crate::state::{lock, AppState, Drag, UsageCmd};
use crate::windows::{self, layout_widget, screen_geometry, DASHBOARD, WIDGET};
use crate::{hotkeys, menu};
use agentmeter_core::settings::Pos;
use agentmeter_core::time::now_ms;
use serde_json::{json, Value};
use std::time::Duration;
use tauri::{AppHandle, LogicalPosition, Manager, State, WebviewWindow};
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub fn app_init(app: AppHandle, window: WebviewWindow, st: State<AppState>) -> Value {
    let page = if window.label() == DASHBOARD {
        lock(&st.pending_page).take()
    } else {
        None
    };
    let usage = lock(&st.last_usage).clone().unwrap_or_default();
    let (_, band, _) = screen_geometry(&app);
    json!({
        "window": window.label(),
        "settings": st.settings(),
        "usage": usage,
        "system": lock(&st.last_system).clone(),
        "platform": std::env::consts::OS,
        "version": env!("CARGO_PKG_VERSION"),
        "page": page,
        "taskbarHeight": band.h,
        "hotkeyErrors": lock(&st.hotkey_errors).clone(),
        "dataDir": st.data_dir.to_string_lossy(),
    })
}

#[tauri::command]
pub fn settings_set(app: AppHandle, patch: Value) -> Value {
    update_settings(&app, patch)
}

#[tauri::command]
pub fn usage_refresh(st: State<AppState>) {
    st.send_usage(UsageCmd::Refresh { force: true });
}

#[tauri::command]
pub fn open_dashboard(app: AppHandle, page: Option<String>) {
    windows::open_dashboard(&app, page);
}

#[tauri::command]
pub fn close_window(app: AppHandle, window: WebviewWindow) {
    if window.label() == WIDGET {
        update_settings(&app, json!({ "widgetVisible": false }));
    } else {
        let _ = window.close();
    }
}

#[tauri::command]
pub fn minimize_window(window: WebviewWindow) {
    let _ = window.minimize();
}

#[tauri::command]
pub fn toggle_maximize(window: WebviewWindow) {
    if window.is_maximized().unwrap_or(false) {
        let _ = window.unmaximize();
    } else {
        let _ = window.maximize();
    }
}

/// The widget sizes itself to its content.
#[tauri::command]
pub fn widget_resize(app: AppHandle, width: f64, height: f64) {
    layout_widget(&app, Some((width.ceil(), height.ceil())));
}

#[tauri::command]
pub fn widget_menu(app: AppHandle, window: WebviewWindow) {
    if let Ok(m) = menu::widget_menu(&app) {
        let _ = window.popup_menu(&m);
    }
}

#[tauri::command]
pub fn drag_start(window: WebviewWindow, st: State<AppState>, x: f64, y: f64) {
    let sf = window.scale_factor().unwrap_or(1.0);
    if let Ok(p) = window.outer_position() {
        *lock(&st.drag) = Some(Drag {
            sx: x,
            sy: y,
            x: p.x as f64 / sf,
            y: p.y as f64 / sf,
        });
    }
}

#[tauri::command]
pub fn drag_move(window: WebviewWindow, st: State<AppState>, x: f64, y: f64) {
    let Some(d) = *lock(&st.drag) else { return };
    let taskbar = st.settings().is_taskbar();
    // docked: slide along the taskbar only
    let ny = if taskbar { d.y } else { d.y + y - d.sy };
    let _ = window.set_position(LogicalPosition::new((d.x + x - d.sx).round(), ny.round()));
}

#[tauri::command]
pub fn drag_end(app: AppHandle, window: WebviewWindow, st: State<AppState>) {
    if lock(&st.drag).take().is_none() {
        return;
    }
    let sf = window.scale_factor().unwrap_or(1.0);
    let (Ok(p), Ok(size)) = (window.outer_position(), window.inner_size()) else {
        return;
    };
    let (x, y, w) = (p.x as f64 / sf, p.y as f64 / sf, size.width as f64 / sf);
    if st.settings().is_taskbar() {
        let (_, band, _) = screen_geometry(&app);
        let offset = (band.x + band.w - (x + w)).max(0.0);
        update_settings(&app, json!({ "taskbarOffset": offset.round() }));
    } else {
        update_settings(
            &app,
            json!({ "floatingPos": Pos { x: x.round(), y: y.round() } }),
        );
    }
}

/// Pause global shortcuts while the dashboard records a new one.
#[tauri::command]
pub fn hotkeys_suspend(app: AppHandle, suspend: bool) {
    if suspend {
        hotkeys::suspend(&app);
    } else {
        hotkeys::register(&app);
    }
}

/// Write the daily history as CSV into <data dir>/exports and reveal it.
#[tauri::command]
pub fn export_csv(app: AppHandle, st: State<AppState>) -> Result<String, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    st.send_usage(UsageCmd::ExportCsv(tx));
    let csv = rx
        .recv_timeout(Duration::from_secs(10))
        .map_err(|_| "내보내기 실패".to_string())?;
    let dir = st.data_dir.join("exports");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let name = format!(
        "agentmeter-usage-{}.csv",
        agentmeter_core::time::day_key(now_ms())
    );
    let path = dir.join(name);
    std::fs::write(&path, csv).map_err(|e| e.to_string())?;
    let _ = app.opener().reveal_item_in_dir(&path);
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_data_dir(app: AppHandle, st: State<AppState>) {
    let _ = app
        .opener()
        .open_path(st.data_dir.to_string_lossy().to_string(), None::<&str>);
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) {
    if url.starts_with("https://") {
        let _ = app.opener().open_url(url, None::<&str>);
    }
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    crate::quit(&app);
}

pub fn handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        app_init,
        settings_set,
        usage_refresh,
        open_dashboard,
        close_window,
        minimize_window,
        toggle_maximize,
        widget_resize,
        widget_menu,
        drag_start,
        drag_move,
        drag_end,
        hotkeys_suspend,
        export_csv,
        open_data_dir,
        open_url,
        quit_app
    ]
}

#[allow(dead_code)]
fn _assert_manager(app: &AppHandle) {
    let _ = app.state::<AppState>();
}
