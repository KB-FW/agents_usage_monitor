//! AgentPets — Tauri shell around `agentpets-core`.

mod commands;
mod http;
mod menu;
mod settings_apply;
mod state;
mod windows;
mod workers;

use state::{lock, AppState, UsageCmd};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{AppHandle, Manager, RunEvent};

/// "2시간 13분" style countdown for notifications.
pub fn fmt_left(ms: i64) -> String {
    let m = (ms.max(0) + 59_999) / 60_000;
    let (d, h, mm) = (m / 1440, (m % 1440) / 60, m % 60);
    if d > 0 {
        format!("{d}일 {h}시간")
    } else if h > 0 {
        format!("{h}시간 {mm}분")
    } else {
        format!("{mm}분")
    }
}

/// Flush everything and exit.
pub fn quit(app: &AppHandle) {
    let st = app.state::<AppState>();
    st.quitting.store(true, Ordering::SeqCst);
    let (tx, rx) = std::sync::mpsc::channel();
    st.send_usage(UsageCmd::Stop(tx));
    let _ = rx.recv_timeout(Duration::from_secs(2));
    st.save_settings();
    state::push_game(app);
    app.exit(0);
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            windows::open_panel(app, None)
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(commands::handler())
        .on_menu_event(|app, e| menu::handle_menu(app, e.id().as_ref()))
        .setup(|app| {
            let handle = app.handle().clone();
            let data_dir = app.path().app_data_dir()?;
            let (state, first_run) = AppState::load(data_dir);
            {
                let mut g = lock(&state.game);
                let mut rng = lock(&state.rng);
                g.tick(agentpets_core::time::now_ms(), &mut *rng);
            }
            app.manage(state);
            state::push_game(&handle);

            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            if let Err(e) = menu::create_tray(&handle) {
                eprintln!("[tray] {e}");
            }
            let s = app.state::<AppState>().settings();
            if s.show_pet {
                windows::create_pet(&handle)?;
            }
            if s.show_bar {
                windows::create_bar(&handle)?;
            }
            if first_run {
                windows::open_panel(&handle, Some("usage".into()));
            }
            workers::spawn_usage(handle.clone());
            workers::spawn_ticker(handle.clone());
            workers::spawn_system(handle.clone());
            workers::spawn_hit_test(handle);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building AgentPets")
        .run(|app, event| {
            // Keep living in the tray when every window is closed.
            if let RunEvent::ExitRequested { api, code, .. } = event {
                if code.is_none() && !app.state::<AppState>().quitting.load(Ordering::SeqCst) {
                    api.prevent_exit();
                }
            }
        });
}
