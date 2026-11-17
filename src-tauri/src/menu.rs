//! Tray icon, tray menu and the pet's right-click menu.

use crate::settings_apply::update_settings;
use crate::state::{lock, AppState, UsageCmd};
use crate::windows::open_panel;
use serde_json::json;
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuBuilder, MenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

const TRAY_ID: &str = "agentpets";

fn item(app: &AppHandle, id: &str, label: &str) -> tauri::Result<MenuItem<Wry>> {
    MenuItem::with_id(app, id, label, true, None::<&str>)
}

fn check(
    app: &AppHandle,
    id: &str,
    label: &str,
    checked: bool,
) -> tauri::Result<CheckMenuItem<Wry>> {
    CheckMenuItem::with_id(app, id, label, true, checked, None::<&str>)
}

pub fn tray_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let s = app.state::<AppState>().settings();
    MenuBuilder::new(app)
        .item(&item(app, "panel:inventory", "패널 열기")?)
        .item(&item(app, "panel:usage", "사용량 보기")?)
        .separator()
        .item(&check(app, "toggle:showPet", "데스크톱 펫", s.show_pet)?)
        .item(&check(app, "toggle:showBar", "사용량 바", s.show_bar)?)
        .item(&item(app, "refresh", "지금 새로고침")?)
        .separator()
        .item(&item(app, "quit", "종료")?)
        .build()
}

pub fn pet_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let s = app.state::<AppState>().settings();
    let scale = Submenu::with_items(
        app,
        "펫 크기",
        true,
        &[
            &check(app, "scale:0.75", "작게", (s.pet_scale - 0.75).abs() < 0.01)?,
            &check(app, "scale:1", "보통", (s.pet_scale - 1.0).abs() < 0.01)?,
            &check(app, "scale:1.25", "크게", (s.pet_scale - 1.25).abs() < 0.01)?,
        ],
    )?;
    MenuBuilder::new(app)
        .item(&item(app, "panel:inventory", "인벤토리")?)
        .item(&item(app, "panel:gacha", "뽑기")?)
        .item(&item(app, "panel:usage", "사용량")?)
        .item(&item(app, "panel:settings", "설정")?)
        .separator()
        .item(&check(app, "toggle:showBar", "사용량 바 표시", s.show_bar)?)
        .item(&check(
            app,
            "toggle:showGauges",
            "게이지 표시",
            s.show_gauges,
        )?)
        .item(&scale)
        .item(&check(
            app,
            "toggle:alwaysOnTop",
            "항상 위에",
            s.always_on_top,
        )?)
        .item(&item(app, "refresh", "지금 새로고침")?)
        .separator()
        .item(&item(app, "hide-pet", "펫 숨기기")?)
        .item(&item(app, "quit", "종료")?)
        .build()
}

pub fn handle_menu(app: &AppHandle, id: &str) {
    let st = app.state::<AppState>();
    if let Some(tab) = id.strip_prefix("panel:") {
        open_panel(app, Some(tab.to_string()));
    } else if let Some(key) = id.strip_prefix("toggle:") {
        let s = serde_json::to_value(st.settings()).unwrap_or_default();
        let cur = s.get(key).and_then(|v| v.as_bool()).unwrap_or(false);
        update_settings(app, json!({ key: !cur }));
    } else if let Some(v) = id.strip_prefix("scale:") {
        if let Ok(scale) = v.parse::<f64>() {
            update_settings(app, json!({ "petScale": scale }));
        }
    } else {
        match id {
            "refresh" => st.send_usage(UsageCmd::Refresh { force: true }),
            "hide-pet" => {
                update_settings(app, json!({ "showPet": false }));
            }
            "quit" => crate::quit(app),
            _ => {}
        }
    }
}

pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let icon = Image::from_bytes(include_bytes!("../icons/tray@2x.png"))
        .ok()
        .or_else(|| app.default_window_icon().cloned())
        .expect("tray icon");
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("AgentPets")
        .menu(&tray_menu(app)?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| handle_menu(app, e.id().as_ref()))
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = e
            {
                open_panel(tray.app_handle(), None);
            }
        })
        .build(app)?;
    Ok(())
}

/// Rebuild the tray menu (checkbox states) and the usage tooltip.
pub fn refresh_tray(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    if let Ok(m) = tray_menu(app) {
        let _ = tray.set_menu(Some(m));
    }
    let st = app.state::<AppState>();
    let usage = lock(&st.last_usage);
    if let Some(u) = usage.as_ref() {
        let mut lines = vec!["AgentPets".to_string()];
        for p in u
            .providers
            .iter()
            .filter(|p| p.enabled && !p.windows.is_empty())
        {
            let ws: Vec<String> = p
                .windows
                .iter()
                .filter(|w| !w.minor)
                .map(|w| format!("{} {}%", w.label, w.used_percent.round()))
                .collect();
            lines.push(format!("{} {}", p.name, ws.join(" · ")));
        }
        let text: String = lines.join("\n").chars().take(127).collect();
        let _ = tray.set_tooltip(Some(text));
    }
}
