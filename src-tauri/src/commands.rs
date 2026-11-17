//! Commands invoked from the UI (`window.agentPets` → ui/shared/bridge.js).

use crate::settings_apply::update_settings;
use crate::state::{emit_event, lock, push_game, AppState, Drag, UsageCmd};
use crate::windows;
use agentpets_core::catalog::catalog_value;
use agentpets_core::game::{self, GameState};
use agentpets_core::settings::Pos;
use agentpets_core::time::now_ms;
use serde_json::{json, Value};
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, State, WebviewWindow};
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub fn app_init(window: WebviewWindow, st: State<AppState>) -> Value {
    let tab = if window.label() == "panel" {
        lock(&st.pending_tab).take()
    } else {
        None
    };
    let usage = lock(&st.last_usage).clone().unwrap_or_default();
    json!({
        "settings": st.settings(),
        "game": lock(&st.game).to_value(),
        "usage": usage,
        "system": lock(&st.last_system).clone(),
        "platform": std::env::consts::OS,
        "version": env!("CARGO_PKG_VERSION"),
        "catalog": catalog_value(),
        "tab": tab,
        "gameConfig": {
            "gachaCost": game::GACHA_COST,
            "gacha10Cost": game::GACHA10_COST,
            "pity3": game::PITY3,
            "pity4": game::PITY4,
            "rates": game::GACHA_RATES,
        },
    })
}

#[tauri::command]
pub fn game_action(app: AppHandle, st: State<AppState>, kind: String, payload: Value) -> Value {
    let now = now_ms();
    let (res, changed) = {
        let mut game = lock(&st.game);
        let mut rng = lock(&st.rng);
        game.action(&kind, &payload, now, &mut *rng)
    };
    match res {
        Err(e) => json!({ "error": e }),
        Ok(v) => {
            if changed {
                push_game(&app);
            }
            let ev = |t: &str| {
                let mut e = v.clone();
                if let Some(o) = e.as_object_mut() {
                    o.insert("type".into(), json!(t));
                }
                e
            };
            match kind.as_str() {
                "gacha" => emit_event(&app, json!({ "type": "gacha", "results": v["results"] })),
                "claimBubble" => emit_event(&app, ev("drop")),
                "synth" => emit_event(&app, ev("synth")),
                "pet" => {
                    let mut e = ev("petted");
                    e["pet"] = payload["pet"].clone();
                    emit_event(&app, e);
                }
                "emote" => emit_event(
                    &app,
                    json!({ "type": "emote", "pet": payload["pet"], "emote": payload["emote"] }),
                ),
                _ => {}
            }
            v
        }
    }
}

#[tauri::command]
pub fn game_reset(app: AppHandle, st: State<AppState>) -> bool {
    let now = now_ms();
    {
        let mut g = lock(&st.game);
        *g = GameState::new(now);
        let mut rng = lock(&st.rng);
        g.tick(now, &mut *rng);
    }
    st.send_usage(UsageCmd::SetCreditSince(now));
    push_game(&app);
    true
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
pub fn open_panel(app: AppHandle, tab: Option<String>) {
    windows::open_panel(&app, tab);
}

#[tauri::command]
pub fn close_window(app: AppHandle, window: WebviewWindow) {
    match window.label() {
        "pet" => {
            update_settings(&app, json!({ "showPet": false }));
        }
        "bar" => {
            update_settings(&app, json!({ "showBar": false }));
        }
        _ => {
            let _ = window.close();
        }
    }
}

#[tauri::command]
pub fn minimize_window(window: WebviewWindow) {
    let _ = window.minimize();
}

/// The usage bar sizes its width to the content (left edge and height stay put, so the
/// window never drifts even where the platform enforces a minimum height).
#[tauri::command]
pub fn resize_window(window: WebviewWindow, width: f64, _height: f64) {
    if window.label() != "bar" {
        return;
    }
    let sf = window.scale_factor().unwrap_or(1.0);
    let Ok(size) = window.inner_size() else {
        return;
    };
    let w = width.clamp(120.0, 1600.0).round();
    let (ow, oh) = (size.width as f64 / sf, size.height as f64 / sf);
    if (ow - w).abs() < 1.0 {
        return;
    }
    let _ = window.set_size(LogicalSize::new(w, oh));
}

#[tauri::command]
pub fn set_hit_regions(st: State<AppState>, rects: Vec<[f64; 4]>) {
    *lock(&st.hit_regions) = rects;
}

#[tauri::command]
pub fn pet_context_menu(app: AppHandle, window: WebviewWindow) {
    if let Ok(m) = crate::menu::pet_menu(&app) {
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
        let _ = window.set_ignore_cursor_events(false);
    }
}

#[tauri::command]
pub fn drag_move(window: WebviewWindow, st: State<AppState>, x: f64, y: f64) {
    if let Some(d) = *lock(&st.drag) {
        let _ = window.set_position(LogicalPosition::new(
            (d.x + x - d.sx).round(),
            (d.y + y - d.sy).round(),
        ));
    }
}

#[tauri::command]
pub fn drag_end(window: WebviewWindow, st: State<AppState>) {
    if lock(&st.drag).take().is_none() {
        return;
    }
    let sf = window.scale_factor().unwrap_or(1.0);
    if let Ok(p) = window.outer_position() {
        let pos = Some(Pos {
            x: p.x as f64 / sf,
            y: p.y as f64 / sf,
        });
        {
            let mut s = lock(&st.settings);
            match window.label() {
                "pet" => s.pet_bounds = pos,
                "bar" => s.bar_bounds = pos,
                _ => {}
            }
        }
        st.save_settings();
    }
}

#[tauri::command]
pub fn open_data_dir(app: AppHandle, st: State<AppState>) {
    let _ = app
        .opener()
        .open_path(st.data_dir.to_string_lossy().to_string(), None::<&str>);
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    crate::quit(&app);
}

pub fn handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        app_init,
        game_action,
        game_reset,
        settings_set,
        usage_refresh,
        open_panel,
        close_window,
        minimize_window,
        resize_window,
        set_hit_regions,
        pet_context_menu,
        drag_start,
        drag_move,
        drag_end,
        open_data_dir,
        quit_app
    ]
}

#[allow(dead_code)]
fn _assert_manager(app: &AppHandle) {
    let _ = app.state::<AppState>();
}
