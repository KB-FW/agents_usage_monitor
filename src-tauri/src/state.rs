//! Shared application state and persistence helpers.

use agentpets_core::game::{GameState, XorShift};
use agentpets_core::settings::Settings;
use agentpets_core::time::now_ms;
use agentpets_core::usage::Snapshot;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::{Mutex, MutexGuard};
use tauri::{AppHandle, Emitter, Manager};

pub enum UsageCmd {
    Refresh { force: bool },
    SetCreditSince(i64),
    Stop(std::sync::mpsc::Sender<()>),
}

#[derive(Clone, Copy)]
pub struct Drag {
    pub sx: f64,
    pub sy: f64,
    pub x: f64,
    pub y: f64,
}

pub struct AppState {
    pub data_dir: PathBuf,
    pub settings: Mutex<Settings>,
    pub game: Mutex<GameState>,
    pub last_usage: Mutex<Option<Snapshot>>,
    pub last_system: Mutex<Value>,
    /// `provider:window` → (last percent, thresholds crossed)
    pub alerts: Mutex<HashMap<String, (f64, usize)>>,
    /// Interactive rectangles of the pet window in CSS px: [x, y, w, h]
    pub hit_regions: Mutex<Vec<[f64; 4]>>,
    pub drag: Mutex<Option<Drag>>,
    pub pending_tab: Mutex<Option<String>>,
    pub usage_tx: Mutex<Option<Sender<UsageCmd>>>,
    pub rng: Mutex<XorShift>,
    pub quitting: AtomicBool,
}

/// Lock that survives a poisoned mutex (a panic elsewhere must not brick the UI).
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn read_json(path: &Path) -> Option<Value> {
    let bytes = std::fs::read(path)
        .ok()
        .or_else(|| std::fs::read(path.with_extension("json.bak")).ok())?;
    serde_json::from_slice(&bytes).ok()
}

pub fn write_json(path: &Path, v: &Value) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let tmp = path.with_extension("json.tmp");
    let Ok(bytes) = serde_json::to_vec_pretty(v) else {
        return;
    };
    if std::fs::write(&tmp, bytes).is_ok() {
        if path.exists() {
            let _ = std::fs::copy(path, path.with_extension("json.bak"));
        }
        let _ = std::fs::rename(&tmp, path);
    }
}

impl AppState {
    pub fn load(data_dir: PathBuf) -> (Self, bool) {
        let settings = Settings::from_value(read_json(&data_dir.join("settings.json")));
        let raw_game = read_json(&data_dir.join("game.json"));
        let first_run = raw_game.is_none();
        let game = GameState::from_value(raw_game, now_ms());
        let seed = now_ms() as u64 ^ 0x9E37_79B9_7F4A_7C15;
        (
            Self {
                data_dir,
                settings: Mutex::new(settings),
                game: Mutex::new(game),
                last_usage: Mutex::new(None),
                last_system: Mutex::new(json!({ "cpu": 0, "ram": 0 })),
                alerts: Mutex::new(HashMap::new()),
                hit_regions: Mutex::new(vec![]),
                drag: Mutex::new(None),
                pending_tab: Mutex::new(None),
                usage_tx: Mutex::new(None),
                rng: Mutex::new(XorShift::new(seed)),
                quitting: AtomicBool::new(false),
            },
            first_run,
        )
    }

    pub fn settings(&self) -> Settings {
        lock(&self.settings).clone()
    }

    pub fn save_settings(&self) {
        let v = serde_json::to_value(&*lock(&self.settings)).unwrap_or(Value::Null);
        write_json(&self.data_dir.join("settings.json"), &v);
    }

    pub fn send_usage(&self, cmd: UsageCmd) {
        if let Some(tx) = lock(&self.usage_tx).as_ref() {
            let _ = tx.send(cmd);
        }
    }
}

/// Save the game and broadcast it to every window.
pub fn push_game(app: &AppHandle) {
    let st = app.state::<AppState>();
    let v = lock(&st.game).to_value();
    write_json(&st.data_dir.join("game.json"), &v);
    let _ = app.emit("game", v);
}

pub fn emit_event(app: &AppHandle, ev: Value) {
    let _ = app.emit("event", ev);
}
