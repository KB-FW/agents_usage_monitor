//! User settings (persisted as JSON; field names match the UI).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const PROVIDER_IDS: [&str; 4] = ["claude", "codex", "gemini", "grok"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ProviderConfig {
    pub enabled: bool,
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
            use_api: true,
            extra_paths: String::new(),
            budget5h: 0.0,
            budget_week: 0.0,
            command: String::new(),
        }
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
    pub show_pet: bool,
    pub show_bar: bool,
    pub pet_scale: f64,
    pub always_on_top: bool,
    pub click_through: bool,
    pub launch_at_login: bool,
    pub notifications: bool,
    pub alert_thresholds: Vec<f64>,
    pub show_system: bool,
    pub show_gauges: bool,
    pub chatter: bool,
    pub show_undetected: bool,
    pub hidden_pets: Vec<String>,
    pub poll_logs_sec: u64,
    pub poll_api_sec: u64,
    pub pet_bounds: Option<Pos>,
    pub bar_bounds: Option<Pos>,
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
            v: 1,
            show_pet: true,
            show_bar: false,
            pet_scale: 1.0,
            always_on_top: true,
            click_through: true,
            launch_at_login: false,
            notifications: true,
            alert_thresholds: vec![80.0, 95.0],
            show_system: true,
            show_gauges: true,
            chatter: true,
            show_undetected: false,
            hidden_pets: vec![],
            poll_logs_sec: 15,
            poll_api_sec: 180,
            pet_bounds: None,
            bar_bounds: None,
            providers,
        }
    }
}

impl Settings {
    pub fn provider(&self, id: &str) -> ProviderConfig {
        self.providers.get(id).cloned().unwrap_or_default()
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
        let mut s = Self::from_value(Some(base));
        s.sanitize();
        s
    }

    fn sanitize(&mut self) {
        if !(0.5..=2.0).contains(&self.pet_scale) {
            self.pet_scale = 1.0;
        }
        self.poll_logs_sec = self.poll_logs_sec.clamp(5, 600);
        self.poll_api_sec = self.poll_api_sec.clamp(60, 3600);
        self.alert_thresholds.retain(|t| *t > 0.0 && *t <= 100.0);
        self.alert_thresholds
            .sort_by(|a, b| a.partial_cmp(b).unwrap());
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
        let s = Settings::from_value(Some(
            json!({ "petScale": 1.25, "providers": { "claude": { "enabled": false } } }),
        ));
        assert_eq!(s.pet_scale, 1.25);
        assert!(!s.provider("claude").enabled);
        assert!(s.provider("claude").use_api, "missing fields keep defaults");
        assert!(s.providers.contains_key("grok"));

        let m = s.merged(
            &json!({ "providers": { "grok": { "budget5h": 1000 } }, "alertThresholds": [95, 70] }),
        );
        assert_eq!(m.provider("grok").budget5h, 1000.0);
        assert!(!m.provider("claude").enabled, "untouched provider kept");
        assert_eq!(m.alert_thresholds, vec![70.0, 95.0]);
        let json = serde_json::to_value(&m).unwrap();
        assert!(json.get("showPet").is_some());
        assert!(json["providers"]["grok"].get("budgetWeek").is_some());
    }
}
