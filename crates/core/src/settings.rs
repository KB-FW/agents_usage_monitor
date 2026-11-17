//! User settings (persisted as JSON; field names match the UI).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const PROVIDER_IDS: [&str; 4] = ["claude", "codex", "gemini", "grok"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ProviderConfig {
    /// Collect usage for this agent at all.
    pub enabled: bool,
    /// Show it in the widget (it stays in the dashboard either way).
    pub visible: bool,
    /// Window ids not to show in the widget (e.g. "7d_opus", "flash").
    pub hidden_windows: Vec<String>,
    /// Use the official usage endpoint where one exists (Claude, Gemini).
    pub use_api: bool,
    /// Extra folders to scan, separated by newlines / commas (e.g. WSL homes).
    pub extra_paths: String,
    /// Token budgets for estimated windows when no official limit is available.
    pub budget5h: f64,
    pub budget_week: f64,
    /// A command printing `{ "windows": [{ "label", "usedPercent", "resetsAt" }] }`.
    pub command: String,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            visible: true,
            hidden_windows: vec![],
            use_api: true,
            extra_paths: String::new(),
            budget5h: 0.0,
            budget_week: 0.0,
            command: String::new(),
        }
    }
}

/// Which parts of the widget are shown. Each mode has its own set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct DisplayOptions {
    pub logo: bool,
    pub name: bool,
    pub label: bool,
    pub bar: bool,
    pub percent: bool,
    pub reset: bool,
    /// Tick on the bar showing how much of the window has elapsed.
    pub pace: bool,
    /// "Runs out before reset" forecast.
    pub forecast: bool,
    pub tokens: bool,
    pub cost: bool,
    pub system: bool,
    /// Model-specific windows (Opus/Sonnet weekly, Gemini Flash...).
    pub minor_windows: bool,
}

impl DisplayOptions {
    pub fn taskbar() -> Self {
        Self {
            logo: true,
            name: false,
            label: true,
            bar: true,
            percent: true,
            reset: true,
            pace: false,
            forecast: true,
            tokens: false,
            cost: false,
            system: true,
            minor_windows: false,
        }
    }
    pub fn floating() -> Self {
        Self {
            logo: true,
            name: true,
            label: true,
            bar: true,
            percent: true,
            reset: true,
            pace: true,
            forecast: true,
            tokens: true,
            cost: true,
            system: false,
            minor_windows: false,
        }
    }
}

impl Default for DisplayOptions {
    fn default() -> Self {
        Self::floating()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Pos {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub v: u32,
    /// "taskbar" (docked on the taskbar) or "floating" (movable card).
    pub mode: String,
    pub widget_visible: bool,
    /// Global shortcuts (accelerator strings, empty = off).
    pub hotkey_mode: String,
    pub hotkey_visible: String,
    pub hotkey_dashboard: String,
    /// "used" or "remaining".
    pub value_mode: String,
    /// "relative" (2h 13m) or "clock" (14:30).
    pub reset_format: String,
    pub taskbar: DisplayOptions,
    pub floating: DisplayOptions,
    /// Widget background opacity 0.3..1.
    pub opacity: f64,
    /// Widget zoom 0.8..1.5.
    pub scale: f64,
    pub always_on_top: bool,
    pub lock_position: bool,
    /// Taskbar mode: distance of the widget's right edge from the monitor's right edge.
    pub taskbar_offset: Option<f64>,
    /// Floating mode: top-left position.
    pub floating_pos: Option<Pos>,
    pub notifications: bool,
    pub alert_thresholds: Vec<f64>,
    pub notify_reset: bool,
    pub notify_forecast: bool,
    /// Draw the highest usage as a gauge in the tray icon.
    pub tray_gauge: bool,
    pub launch_at_login: bool,
    pub poll_logs_sec: u64,
    pub poll_api_sec: u64,
    pub providers: BTreeMap<String, ProviderConfig>,
}

impl Default for Settings {
    fn default() -> Self {
        let providers = PROVIDER_IDS
            .iter()
            .map(|id| {
                let mut c = ProviderConfig::default();
                if *id == "grok" || *id == "codex" {
                    c.use_api = false;
                }
                (id.to_string(), c)
            })
            .collect();
        Self {
            v: 2,
            mode: "taskbar".into(),
            widget_visible: true,
            hotkey_mode: "PageUp".into(),
            hotkey_visible: "Shift+PageUp".into(),
            hotkey_dashboard: String::new(),
            value_mode: "used".into(),
            reset_format: "relative".into(),
            taskbar: DisplayOptions::taskbar(),
            floating: DisplayOptions::floating(),
            opacity: 0.92,
            scale: 1.0,
            always_on_top: true,
            lock_position: false,
            taskbar_offset: None,
            floating_pos: None,
            notifications: true,
            alert_thresholds: vec![80.0, 95.0],
            notify_reset: true,
            notify_forecast: true,
            tray_gauge: true,
            launch_at_login: false,
            poll_logs_sec: 15,
            poll_api_sec: 180,
            providers,
        }
    }
}

impl Settings {
    pub fn provider(&self, id: &str) -> ProviderConfig {
        self.providers.get(id).cloned().unwrap_or_default()
    }

    pub fn is_taskbar(&self) -> bool {
        self.mode != "floating"
    }

    /// Load from JSON, filling in defaults (including providers missing from old files).
    pub fn from_value(v: Option<Value>) -> Self {
        let mut s: Settings = v
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
        for id in PROVIDER_IDS {
            s.providers.entry(id.to_string()).or_default();
        }
        s.sanitize();
        s
    }

    /// Apply a partial update coming from the UI (objects are merged deeply).
    pub fn merged(&self, patch: &Value) -> Self {
        let mut base = serde_json::to_value(self).unwrap_or(Value::Null);
        deep_merge(&mut base, patch);
        Self::from_value(Some(base))
    }

    fn sanitize(&mut self) {
        if self.mode != "taskbar" && self.mode != "floating" {
            self.mode = "taskbar".into();
        }
        if self.value_mode != "used" && self.value_mode != "remaining" {
            self.value_mode = "used".into();
        }
        if self.reset_format != "relative" && self.reset_format != "clock" {
            self.reset_format = "relative".into();
        }
        if !self.opacity.is_finite() {
            self.opacity = 0.92;
        }
        self.opacity = self.opacity.clamp(0.3, 1.0);
        if !(0.8..=1.5).contains(&self.scale) {
            self.scale = 1.0;
        }
        self.poll_logs_sec = self.poll_logs_sec.clamp(5, 600);
        self.poll_api_sec = self.poll_api_sec.clamp(60, 3600);
        self.alert_thresholds.retain(|t| *t > 0.0 && *t <= 100.0);
        self.alert_thresholds
            .sort_by(|a, b| a.partial_cmp(b).unwrap());
        self.alert_thresholds.dedup();
    }
}

pub fn deep_merge(base: &mut Value, patch: &Value) {
    match (base, patch) {
        (Value::Object(b), Value::Object(p)) => {
            for (k, v) in p {
                match b.get_mut(k) {
                    Some(slot) if slot.is_object() && v.is_object() => deep_merge(slot, v),
                    _ => {
                        b.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        (b, p) => *b = p.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_and_merge() {
        let s = Settings::default();
        assert_eq!(s.mode, "taskbar");
        assert_eq!(s.hotkey_mode, "PageUp");
        assert!(s.taskbar.system && !s.floating.system);

        let s = Settings::from_value(Some(
            json!({ "mode": "floating", "providers": { "claude": { "enabled": false } } }),
        ));
        assert_eq!(s.mode, "floating");
        assert!(!s.provider("claude").enabled);
        assert!(s.provider("claude").use_api, "missing fields keep defaults");
        assert!(s.providers.contains_key("grok"));

        let m = s.merged(&json!({
            "providers": { "grok": { "budget5h": 1000, "hiddenWindows": ["7d"] } },
            "floating": { "cost": false },
            "valueMode": "remaining",
            "alertThresholds": [95, 70],
            "opacity": 7
        }));
        assert_eq!(m.provider("grok").budget5h, 1000.0);
        assert_eq!(m.provider("grok").hidden_windows, vec!["7d".to_string()]);
        assert!(!m.provider("claude").enabled, "untouched provider kept");
        assert!(
            !m.floating.cost && m.floating.tokens,
            "nested display options merge"
        );
        assert_eq!(m.value_mode, "remaining");
        assert_eq!(m.alert_thresholds, vec![70.0, 95.0]);
        assert_eq!(m.opacity, 1.0);
        let json = serde_json::to_value(&m).unwrap();
        assert!(json.get("hotkeyMode").is_some());
        assert!(json["taskbar"].get("minorWindows").is_some());
        assert!(json["providers"]["grok"].get("budgetWeek").is_some());

        let bad = s.merged(&json!({ "mode": "sideways", "valueMode": "x" }));
        assert_eq!(bad.mode, "taskbar");
        assert_eq!(bad.value_mode, "used");
    }
}
