//! Token ledger: de-duplicated usage events per provider.
//!
//! * Recent events (lookback window) are stored individually under a provider-specific key,
//!   so re-reads, resumed sessions and rewritten files never double count; re-recording a
//!   key with new numbers applies only the difference.
//! * Daily totals are kept ~120 days for the statistics view.
//! * Deltas for events newer than `credit_since` accumulate in `pending` and feed the game.

use crate::time::{day_key, DAY, HOUR};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Copy, Default)]
pub struct Tokens {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}

/// Cache reads cost ~10% of fresh input, so they count at 10% to keep providers with
/// aggressive prompt caching comparable.
pub fn weigh(t: &Tokens) -> i64 {
    (t.input + t.output + t.cache_write + t.cache_read * 0.1).round() as i64
}

/// `[ts, weighted, input, output, cacheRead, cacheWrite, model]`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventRec(
    pub i64,
    pub i64,
    pub i64,
    pub i64,
    pub i64,
    pub i64,
    pub Option<String>,
);

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct DayTotals {
    pub w: i64,
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
    pub cache_write: i64,
    pub n: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenStats {
    pub today: i64,
    pub today_detail: DayTotals,
    pub last5h: i64,
    pub week: i64,
    pub last_event_at: Option<i64>,
    pub models: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryDay {
    pub day: String,
    pub w: i64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LedgerData {
    pub v: u32,
    pub events: BTreeMap<String, Vec<(String, EventRec)>>,
    pub daily: BTreeMap<String, BTreeMap<String, DayTotals>>,
    pub last_event_at: BTreeMap<String, i64>,
    pub pending: BTreeMap<String, i64>,
}

#[derive(Debug)]
pub struct Ledger {
    pub lookback_ms: i64,
    pub history_days: i64,
    pub credit_since: i64,
    events: HashMap<String, HashMap<String, EventRec>>,
    daily: HashMap<String, BTreeMap<String, DayTotals>>,
    last_event_at: HashMap<String, i64>,
    pending: BTreeMap<String, i64>,
    pub dirty: bool,
}

impl Ledger {
    pub fn new(lookback_days: i64, credit_since: i64) -> Self {
        Self {
            lookback_ms: lookback_days * DAY,
            history_days: 120,
            credit_since,
            events: HashMap::new(),
            daily: HashMap::new(),
            last_event_at: HashMap::new(),
            pending: BTreeMap::new(),
            dirty: false,
        }
    }

    fn add_daily(&mut self, provider: &str, rec: &EventRec, sign: i64) {
        let d = self
            .daily
            .entry(provider.to_string())
            .or_default()
            .entry(day_key(rec.0))
            .or_default();
        d.w += sign * rec.1;
        d.input += sign * rec.2;
        d.output += sign * rec.3;
        d.cache_read += sign * rec.4;
        d.cache_write += sign * rec.5;
        d.n += sign;
    }

    /// Record an event; returns the weighted delta applied (0 for duplicates / stale events).
    pub fn record(
        &mut self,
        provider: &str,
        key: &str,
        ts: i64,
        t: Tokens,
        model: Option<&str>,
        now: i64,
    ) -> i64 {
        if key.is_empty() || ts < now - self.lookback_ms || ts > now + DAY {
            return 0;
        }
        let rec = EventRec(
            ts,
            weigh(&t),
            t.input.round() as i64,
            t.output.round() as i64,
            t.cache_read.round() as i64,
            t.cache_write.round() as i64,
            model.map(|m| m.to_string()),
        );
        if rec.1 <= 0 && rec.2 + rec.3 + rec.4 + rec.5 <= 0 {
            return 0;
        }
        let prev = self.events.get(provider).and_then(|m| m.get(key)).cloned();
        if let Some(p) = &prev {
            if p.1 == rec.1 && p.2 == rec.2 && p.3 == rec.3 {
                return 0;
            }
            self.add_daily(provider, p, -1);
        }
        self.add_daily(provider, &rec, 1);
        let delta = rec.1 - prev.as_ref().map(|p| p.1).unwrap_or(0);
        self.events
            .entry(provider.to_string())
            .or_default()
            .insert(key.to_string(), rec);
        let last = self.last_event_at.entry(provider.to_string()).or_insert(0);
        if ts > *last {
            *last = ts;
        }
        if ts >= self.credit_since && delta > 0 {
            *self.pending.entry(provider.to_string()).or_insert(0) += delta;
        }
        self.dirty = true;
        delta
    }

    pub fn take_pending(&mut self) -> BTreeMap<String, i64> {
        std::mem::take(&mut self.pending)
    }

    pub fn prune(&mut self, now: i64) {
        let cutoff = now - self.lookback_ms;
        for m in self.events.values_mut() {
            m.retain(|_, r| r.0 >= cutoff);
        }
        let day_cutoff = day_key(now - self.history_days * DAY);
        for d in self.daily.values_mut() {
            d.retain(|k, _| *k >= day_cutoff);
        }
    }

    /// Sum of weighted tokens in `[from, to)`.
    pub fn sum(&self, provider: &str, from: i64, to: i64) -> i64 {
        self.events
            .get(provider)
            .map(|m| {
                m.values()
                    .filter(|r| r.0 >= from && r.0 < to)
                    .map(|r| r.1)
                    .sum()
            })
            .unwrap_or(0)
    }

    pub fn stats(&self, provider: &str, now: i64) -> TokenStats {
        let today = self
            .daily
            .get(provider)
            .and_then(|d| d.get(&day_key(now)))
            .cloned()
            .unwrap_or_default();
        let mut models = BTreeMap::new();
        if let Some(m) = self.events.get(provider) {
            for r in m.values() {
                if let Some(model) = &r.6 {
                    if r.0 >= now - 7 * DAY {
                        *models.entry(model.clone()).or_insert(0) += r.1;
                    }
                }
            }
        }
        TokenStats {
            today: today.w,
            today_detail: today,
            last5h: self.sum(provider, now - 5 * HOUR, i64::MAX),
            week: self.sum(provider, now - 7 * DAY, i64::MAX),
            last_event_at: self.last_event_at.get(provider).copied().filter(|t| *t > 0),
            models,
        }
    }

    /// Daily weighted totals for the last `days` days, oldest first.
    pub fn history(&self, provider: &str, days: i64, now: i64) -> Vec<HistoryDay> {
        let by_day = self.daily.get(provider);
        (0..days)
            .rev()
            .map(|i| {
                let k = day_key(now - i * DAY);
                let w = by_day.and_then(|d| d.get(&k)).map(|d| d.w).unwrap_or(0);
                HistoryDay { day: k, w }
            })
            .collect()
    }

    pub fn to_data(&self) -> LedgerData {
        LedgerData {
            v: 1,
            events: self
                .events
                .iter()
                .map(|(p, m)| {
                    (
                        p.clone(),
                        m.iter().map(|(k, r)| (k.clone(), r.clone())).collect(),
                    )
                })
                .collect(),
            daily: self
                .daily
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            last_event_at: self
                .last_event_at
                .iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect(),
            pending: self.pending.clone(),
        }
    }

    pub fn load(&mut self, data: LedgerData) {
        if data.v != 1 {
            return;
        }
        self.events = data
            .events
            .into_iter()
            .map(|(p, v)| (p, v.into_iter().collect()))
            .collect();
        self.daily = data.daily.into_iter().collect();
        self.last_event_at = data.last_event_at.into_iter().collect();
        self.pending = data.pending;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deltas_and_credit_window() {
        let mut l = Ledger::new(8, 1000);
        let now = 10_000;
        let t = |i: f64| Tokens {
            input: i,
            ..Default::default()
        };
        assert_eq!(l.record("x", "k", 500, t(100.0), None, now), 100);
        assert!(l.take_pending().is_empty(), "before credit_since");
        assert_eq!(l.record("x", "k2", 2000, t(100.0), None, now), 100);
        assert_eq!(
            l.record("x", "k2", 2000, t(100.0), None, now),
            0,
            "duplicate"
        );
        assert_eq!(l.record("x", "k2", 2000, t(150.0), None, now), 50, "delta");
        assert_eq!(l.take_pending().get("x"), Some(&150));
        assert_eq!(l.sum("x", 0, i64::MAX), 250);
        assert_eq!(
            weigh(&Tokens {
                input: 1000.0,
                output: 500.0,
                cache_read: 10000.0,
                cache_write: 2000.0
            }),
            4500
        );
    }
}
