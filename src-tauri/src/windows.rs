//! The three windows: the desktop pet, the usage bar and the main panel.

use crate::state::{lock, AppState};
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

pub const PET_W: f64 = 560.0;
pub const PET_H: f64 = 360.0;
pub const BAR_W: f64 = 640.0;
pub const BAR_H: f64 = 58.0;

/// Primary monitor work area in logical px: (x, y, w, h).
fn work_area(app: &AppHandle) -> (f64, f64, f64, f64) {
    if let Ok(Some(m)) = app.primary_monitor() {
        let s = m.scale_factor();
        let wa = m.work_area();
        return (
            wa.position.x as f64 / s,
            wa.position.y as f64 / s,
            wa.size.width as f64 / s,
            wa.size.height as f64 / s,
        );
    }
    (0.0, 0.0, 1280.0, 720.0)
}

/// Keep a window reachable if the saved position is off-screen (monitor unplugged).
fn clamp_to_screen(app: &AppHandle, x: f64, y: f64, w: f64) -> (f64, f64) {
    let monitors = app.available_monitors().unwrap_or_default();
    let visible = monitors.iter().any(|m| {
        let s = m.scale_factor();
        let (mx, my) = (m.position().x as f64 / s, m.position().y as f64 / s);
        let (mw, mh) = (m.size().width as f64 / s, m.size().height as f64 / s);
        x + w - 60.0 > mx && x + 60.0 < mx + mw && y > my - 20.0 && y + 40.0 < my + mh
    });
    if visible || monitors.is_empty() {
        (x, y)
    } else {
        let (ax, ay, aw, ah) = work_area(app);
        (ax + aw - w - 24.0, ay + ah - PET_H)
    }
}

pub fn pet_size(scale: f64) -> (f64, f64) {
    (PET_W * scale, PET_H * scale)
}

pub fn create_pet(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    if let Some(w) = app.get_webview_window("pet") {
        let _ = w.show();
        return Ok(w);
    }
    let st = app.state::<AppState>();
    let s = st.settings();
    let (w, h) = pet_size(s.pet_scale);
    let (ax, ay, aw, ah) = work_area(app);
    let (x, y) = match s.pet_bounds {
        Some(p) => clamp_to_screen(app, p.x, p.y, w),
        None => (ax + aw - w - 24.0, ay + ah - h),
    };
    let win = WebviewWindowBuilder::new(app, "pet", WebviewUrl::App("pet/index.html".into()))
        .title("AgentPets")
        .inner_size(w, h)
        .position(x, y)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .skip_taskbar(true)
        .always_on_top(s.always_on_top)
        .visible_on_all_workspaces(true)
        .focused(false)
        .build()?;
    if s.click_through {
        let _ = win.set_ignore_cursor_events(true);
    }
    Ok(win)
}

pub fn create_bar(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    if let Some(w) = app.get_webview_window("bar") {
        let _ = w.show();
        return Ok(w);
    }
    let s = app.state::<AppState>().settings();
    let (ax, ay, _aw, ah) = work_area(app);
    let (x, y) = match s.bar_bounds {
        Some(p) => clamp_to_screen(app, p.x, p.y, BAR_W),
        None => (ax + 16.0, ay + ah - BAR_H - 8.0),
    };
    WebviewWindowBuilder::new(app, "bar", WebviewUrl::App("bar/index.html".into()))
        .title("AgentPets Usage")
        .inner_size(BAR_W, BAR_H)
        .position(x, y)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .visible_on_all_workspaces(true)
        .focused(false)
        .build()
}

pub fn open_panel(app: &AppHandle, tab: Option<String>) {
    if let Some(w) = app.get_webview_window("panel") {
        if let Some(t) = tab {
            let _ = app.emit_to(
                "panel",
                "event",
                serde_json::json!({ "type": "navigate", "tab": t }),
            );
        }
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    *lock(&app.state::<AppState>().pending_tab) = tab;
    let res = WebviewWindowBuilder::new(app, "panel", WebviewUrl::App("panel/index.html".into()))
        .title("AgentPets")
        .inner_size(1080.0, 720.0)
        .center()
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .maximizable(false)
        .build();
    if let Err(e) = res {
        eprintln!("[panel] {e}");
    }
}

pub fn close(app: &AppHandle, label: &str) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.close();
    }
}

/// Resize the pet window for a new scale, keeping its bottom-right corner in place.
pub fn rescale_pet(app: &AppHandle, scale: f64) {
    let Some(w) = app.get_webview_window("pet") else {
        return;
    };
    let sf = w.scale_factor().unwrap_or(1.0);
    let (Ok(pos), Ok(size)) = (w.outer_position(), w.inner_size()) else {
        return;
    };
    let (x, y) = (pos.x as f64 / sf, pos.y as f64 / sf);
    let (ow, oh) = (size.width as f64 / sf, size.height as f64 / sf);
    let (nw, nh) = pet_size(scale);
    let _ = w.set_size(LogicalSize::new(nw, nh));
    let _ = w.set_position(LogicalPosition::new(x + ow - nw, y + oh - nh));
}
