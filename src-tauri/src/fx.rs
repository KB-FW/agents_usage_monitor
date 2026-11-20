//! USD → KRW rate for showing costs in won. Fetched from open.er-api.com (free, no key)
//! twice a day and cached in the data folder; a rate set in the settings wins over it.

use crate::state::{read_json, AppState};
use agentmeter_core::time::now_ms;
use serde_json::{json, Value};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const URL: &str = "https://open.er-api.com/v6/latest/USD";
const EVERY: Duration = Duration::from_secs(12 * 3600);
const STALE_MS: i64 = 12 * 3600 * 1000;

static RATE: Mutex<Option<(f64, i64)>> = Mutex::new(None);

/// `{ krw, at }` (both null until the first fetch).
pub fn current() -> Value {
    match RATE.lock().ok().and_then(|r| *r) {
        Some((krw, at)) => json!({ "krw": krw, "at": at }),
        None => json!({ "krw": null, "at": null }),
    }
}

fn store(app: &AppHandle, krw: f64, at: i64) {
    if let Ok(mut r) = RATE.lock() {
        *r = Some((krw, at));
    }
    let path = app.state::<AppState>().data_dir.join("fx.json");
    let _ = std::fs::write(path, json!({ "krw": krw, "at": at }).to_string());
    let _ = app.emit("event", json!({ "type": "fx", "fx": current() }));
}

fn fetch() -> Result<f64, String> {
    let body = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(15))
        .try_proxy_from_env(true)
        .build()
        .get(URL)
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    v.pointer("/rates/KRW")
        .and_then(|r| r.as_f64())
        .filter(|r| r.is_finite() && *r > 100.0 && *r < 100_000.0)
        .ok_or_else(|| "환율 응답에 KRW가 없어요".to_string())
}

pub fn spawn(app: AppHandle) {
    let cached = read_json(&app.state::<AppState>().data_dir.join("fx.json"))
        .and_then(|v| Some((v.get("krw")?.as_f64()?, v.get("at")?.as_i64()?)));
    if let Some((krw, at)) = cached {
        if let Ok(mut r) = RATE.lock() {
            *r = Some((krw, at));
        }
    }
    std::thread::spawn(move || loop {
        if cached_at().is_some_and(|at| now_ms() - at < STALE_MS) {
            std::thread::sleep(Duration::from_secs(3600));
            continue;
        }
        match fetch() {
            Ok(krw) => {
                store(&app, krw, now_ms());
                std::thread::sleep(EVERY);
            }
            Err(e) => {
                eprintln!("[fx] {e}");
                // offline or blocked: try again in a while
                std::thread::sleep(Duration::from_secs(30 * 60));
            }
        }
    });
}

fn cached_at() -> Option<i64> {
    RATE.lock().ok().and_then(|r| r.map(|(_, at)| at))
}
