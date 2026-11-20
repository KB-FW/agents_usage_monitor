//! Shared application state and persistence helpers.

use agentmeter_core::settings::Settings;
use agentmeter_core::usage::Snapshot;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::{Mutex, MutexGuard};

pub enum UsageCmd {
    Refresh { force: bool },
    ExportCsv(Sender<String>),
    Stop(Sender<()>),
}

#[derive(Clone, Copy)]
pub struct Drag {
    pub sx: f64,
    pub sy: f64,
    pub x: f64,
    pub y: f64,
    /// A docked strip was pulled out of the taskbar during this drag.
    pub undocked: bool,
}

/// What we already told the user about one usage window.
#[derive(Default, Clone)]
pub struct AlertMemo {
    pub pct: f64,
    pub level: usize,
    /// `resets_at` of the window cycle we already sent a forecast warning for.
    pub forecast_for: Option<i64>,
}

pub struct AppState {
    pub data_dir: PathBuf,
    pub settings: Mutex<Settings>,
    pub last_usage: Mutex<Option<Snapshot>>,
    pub last_system: Mutex<Value>,
    /// `provider:window` → memo
    pub alerts: Mutex<HashMap<String, AlertMemo>>,
    pub drag: Mutex<Option<Drag>>,
    pub pending_page: Mutex<Option<String>>,
    pub usage_tx: Mutex<Option<Sender<UsageCmd>>>,
    pub hotkey_errors: Mutex<Vec<String>>,
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
        let raw = read_json(&data_dir.join("settings.json"));
        let first_run = raw.is_none();
        let settings = Settings::from_value(raw);
        (
            Self {
                data_dir,
                settings: Mutex::new(settings),
                last_usage: Mutex::new(None),
                last_system: Mutex::new(json!({ "cpu": 0, "ram": 0 })),
                alerts: Mutex::new(HashMap::new()),
                drag: Mutex::new(None),
                pending_page: Mutex::new(None),
                usage_tx: Mutex::new(None),
                hotkey_errors: Mutex::new(vec![]),
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
