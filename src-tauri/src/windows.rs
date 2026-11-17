//! The widget (taskbar-docked or floating) and the dashboard window.

use crate::state::{lock, AppState};
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

pub const WIDGET: &str = "widget";
pub const DASHBOARD: &str = "dashboard";

/// Logical rectangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// The primary monitor and its taskbar band (logical px). When the taskbar reserves no
/// space (auto-hide, side taskbar) the band falls back to a 44 px strip at the bottom.
pub fn screen_geometry(app: &AppHandle) -> (Rect, Rect, Rect) {
    let Ok(Some(m)) = app.primary_monitor() else {
        let mon = Rect {
            x: 0.0,
            y: 0.0,
            w: 1280.0,
            h: 720.0,
        };
        return (
            mon,
            Rect {
                y: 676.0,
                h: 44.0,
                ..mon
            },
            Rect { h: 676.0, ..mon },
        );
    };
    let s = m.scale_factor();
    let mon = Rect {
        x: m.position().x as f64 / s,
        y: m.position().y as f64 / s,
        w: m.size().width as f64 / s,
        h: m.size().height as f64 / s,
    };
    let wa = m.work_area();
    let work = Rect {
        x: wa.position.x as f64 / s,
        y: wa.position.y as f64 / s,
        w: wa.size.width as f64 / s,
        h: wa.size.height as f64 / s,
    };
    let bottom = (mon.y + mon.h) - (work.y + work.h);
    let top = work.y - mon.y;
    let band = if bottom >= 24.0 {
        Rect {
            x: mon.x,
            y: work.y + work.h,
            w: mon.w,
            h: bottom,
        }
    } else if top >= 24.0 {
        Rect {
            x: mon.x,
            y: mon.y,
            w: mon.w,
            h: top,
        }
    } else {
        Rect {
            x: mon.x,
            y: mon.y + mon.h - 44.0,
            w: mon.w,
            h: 44.0,
        }
    };
    (mon, band, work)
}

/// Default gap between the widget and the right end of the taskbar (keeps the tray icons
/// and the clock visible).
const DEFAULT_TASKBAR_OFFSET: f64 = 360.0;

fn current_size(w: &WebviewWindow) -> (f64, f64) {
    let sf = w.scale_factor().unwrap_or(1.0);
    w.inner_size()
        .map(|s| (s.width as f64 / sf, s.height as f64 / sf))
        .unwrap_or((320.0, 40.0))
}

/// Place the widget for the current mode, with the given content size (logical px).
pub fn layout_widget(app: &AppHandle, content: Option<(f64, f64)>) {
    let Some(win) = app.get_webview_window(WIDGET) else {
        return;
    };
    let s = app.state::<AppState>().settings();
    let (cw, ch) = content.unwrap_or_else(|| current_size(&win));
    let (mon, band, work) = screen_geometry(app);
    if s.is_taskbar() {
        let w = cw.clamp(60.0, band.w - 40.0);
        let h = band.h;
        let offset = s
            .taskbar_offset
            .unwrap_or(DEFAULT_TASKBAR_OFFSET)
            .clamp(0.0, (band.w - w).max(0.0));
        let x = band.x + band.w - offset - w;
        let _ = win.set_size(LogicalSize::new(w, h));
        let _ = win.set_position(LogicalPosition::new(x.round(), band.y.round()));
    } else {
        let (w, h) = (cw.max(120.0), ch.max(40.0));
        let (mut x, mut y) = match s.floating_pos {
            Some(p) => (p.x, p.y),
            None => (work.x + work.w - w - 24.0, work.y + work.h - h - 24.0),
        };
        // keep it reachable
        x = x.clamp(mon.x - w + 80.0, mon.x + mon.w - 80.0);
        y = y.clamp(mon.y, mon.y + mon.h - 40.0);
        let _ = win.set_size(LogicalSize::new(w, h));
        let _ = win.set_position(LogicalPosition::new(x.round(), y.round()));
    }
    let _ = win.set_always_on_top(s.always_on_top || s.is_taskbar());
}

pub fn create_widget(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    if let Some(w) = app.get_webview_window(WIDGET) {
        let _ = w.show();
        layout_widget(app, None);
        return Ok(w);
    }
    let s = app.state::<AppState>().settings();
    let (_, band, _) = screen_geometry(app);
    let win = WebviewWindowBuilder::new(app, WIDGET, WebviewUrl::App("widget/index.html".into()))
        .title("AgentMeter")
        // start small: the page reports its real size right away, and GTK won't shrink a
        // window below the size it was created with
        .inner_size(120.0, if s.is_taskbar() { band.h } else { 40.0 })
        .position(band.x + band.w - 780.0, band.y)
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
        .build()?;
    layout_widget(app, None);
    Ok(win)
}

pub fn hide_widget(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(WIDGET) {
        let _ = w.close();
    }
}

pub fn open_dashboard(app: &AppHandle, page: Option<String>) {
    if let Some(w) = app.get_webview_window(DASHBOARD) {
        if let Some(p) = page {
            let _ = app.emit_to(
                DASHBOARD,
                "event",
                serde_json::json!({ "type": "navigate", "page": p }),
            );
        }
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    *lock(&app.state::<AppState>().pending_page) = page;
    let res = WebviewWindowBuilder::new(
        app,
        DASHBOARD,
        WebviewUrl::App("dashboard/index.html".into()),
    )
    .title("AgentMeter")
    .inner_size(1060.0, 720.0)
    .min_inner_size(900.0, 600.0)
    .center()
    .decorations(false)
    .shadow(true)
    .resizable(true)
    .build();
    if let Err(e) = res {
        eprintln!("[dashboard] {e}");
    }
}

pub fn toggle_dashboard(app: &AppHandle) {
    match app.get_webview_window(DASHBOARD) {
        Some(w) if w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false) => {
            let _ = w.close();
        }
        _ => open_dashboard(app, None),
    }
}

/// Re-assert "topmost" so the docked widget stays above the taskbar after the taskbar is
/// clicked (Windows raises it).
pub fn keep_on_top(win: &WebviewWindow) {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        };
        if let Ok(hwnd) = win.hwnd() {
            unsafe {
                SetWindowPos(
                    hwnd.0 as _,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
                );
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = win.set_always_on_top(true);
    }
}
