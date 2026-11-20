//! Token ledger: de-duplicated usage events per provider.
//!
//! * Recent events (lookback window) are stored individually under a provider-specific key,
//!   so re-reads, resumed sessions and rewritten files never double count; re-recording a
//!   key with new numbers applies only the difference.
//! * Each event carries its model, project and estimated API cost, which feed the model /
//!   project breakdowns, the hour-of-day heatmap and the cost figures.
//! * Daily totals (tokens + cost) are kept ~120 days for the history charts.

use crate::pricing;
use crate::time::{day_key, local_date, local_hour, DAY, HOUR};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Copy, Default)]
pub struct Tokens {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}

/// Cache reads cost a fraction of fresh input, so they count at 10% to keep providers with
/// aggressive prompt caching comparable.
pub fn weigh(t: &Tokens) -> i64 {
    (t.input + t.output + t.cache_write + t.cache_read * 0.1).round() as i64
}

/// One usage event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventRec {
    pub ts: i64,
    pub w: i64,
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
    pub cache_write: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// Estimated USD at API list prices; `None` when the model has no known price.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct DayTotals {
    pub w: i64,
    pub input: i64,
    pub output: i64,
    pub cache_read: i64,
    pub cache_write: i64,
    pub n: i64,
    pub cost: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Breakdown {
    pub name: String,
    pub w: i64,
    pub cost: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenStats {
    pub today: i64,
    pub today_detail: DayTotals,
    pub last5h: i64,
    pub week: i64,
    pub cost_today: f64,
    pub cost5h: f64,
    pub cost_week: f64,
    pub cost30d: f64,
    /// Weighted tokens in the last 7 days whose model has no known price.
    pub unpriced_week: i64,
    pub last_event_at: Option<i64>,
    /// Top models / projects over the last 7 days.
    pub models: Vec<Breakdown>,
    pub projects: Vec<Breakdown>,
    /// 7 rows (6 days ago → today) × 24 local hours of weighted tokens.
    pub heatmap: Vec<Vec<i64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryDay {
    pub day: String,
    pub w: i64,
    pub cost: f64,
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

const LEDGER_VERSION: u32 = 2;

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
            .entry(day_key(rec.ts))
            .or_default();
        d.w += sign * rec.w;
        d.input += sign * rec.input;
        d.output += sign * rec.output;
        d.cache_read += sign * rec.cache_read;
        d.cache_write += sign * rec.cache_write;
        d.n += sign;
        d.cost += sign as f64 * rec.cost.unwrap_or(0.0);
    }

    /// Like [`Ledger::record`], for work the agent started on its own (Codex memory
    /// consolidation, command reviews): counted in the totals, but not as user activity.
    #[allow(clippy::too_many_arguments)]
    pub fn record_background(
        &mut self,
        provider: &str,
        key: &str,
        ts: i64,
        t: Tokens,
        model: Option<&str>,
        project: Option<&str>,
        now: i64,
    ) -> i64 {
        let prev = self.last_event_at.get(provider).copied();
        let delta = self.record(provider, key, ts, t, model, project, now);
        match prev {
            Some(p) => {
                self.last_event_at.insert(provider.to_string(), p);
            }
            None => {
                self.last_event_at.remove(provider);
            }
        }
        delta
    }

    /// Record an event; returns the weighted delta applied (0 for duplicates / stale events).
    #[allow(clippy::too_many_arguments)]
    pub fn record(
        &mut self,
        provider: &str,
        key: &str,
        ts: i64,
        t: Tokens,
        model: Option<&str>,
        project: Option<&str>,
        now: i64,
    ) -> i64 {
        if key.is_empty() || ts < now - self.lookback_ms || ts > now + DAY {
            return 0;
        }
        let rec = EventRec {
            ts,
            w: weigh(&t),
            input: t.input.round() as i64,
            output: t.output.round() as i64,
            cache_read: t.cache_read.round() as i64,
            cache_write: t.cache_write.round() as i64,
            model: model.map(String::from),
            project: project.filter(|p| !p.is_empty()).map(String::from),
            cost: pricing::cost(provider, model, &t),
        };
        if rec.w <= 0 && rec.input + rec.output + rec.cache_read + rec.cache_write <= 0 {
            return 0;
        }
        let prev = self.events.get(provider).and_then(|m| m.get(key)).cloned();
        if let Some(p) = &prev {
            if p.w == rec.w && p.input == rec.input && p.output == rec.output {
                return 0;
            }
            self.add_daily(provider, p, -1);
        }
        self.add_daily(provider, &rec, 1);
        let delta = rec.w - prev.as_ref().map(|p| p.w).unwrap_or(0);
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

    /// Weighted tokens recorded since the previous call (used for "tokens are flowing").
    /// Time of the newest user event recorded for a provider.
    pub fn last_event(&self, provider: &str) -> Option<i64> {
        self.last_event_at.get(provider).copied().filter(|t| *t > 0)
    }

    pub fn take_pending(&mut self) -> BTreeMap<String, i64> {
        std::mem::take(&mut self.pending)
    }

    pub fn prune(&mut self, now: i64) {
        let cutoff = now - self.lookback_ms;
        for m in self.events.values_mut() {
            m.retain(|_, r| r.ts >= cutoff);
        }
        let day_cutoff = day_key(now - self.history_days * DAY);
        for d in self.daily.values_mut() {
            d.retain(|k, _| *k >= day_cutoff);
        }
    }

    fn events_since<'a>(
        &'a self,
        provider: &str,
        from: i64,
    ) -> impl Iterator<Item = &'a EventRec> + 'a {
        self.events
            .get(provider)
            .into_iter()
            .flat_map(|m| m.values())
            .filter(move |r| r.ts >= from)
    }

    /// Sum of weighted tokens in `[from, to)`.
    pub fn sum(&self, provider: &str, from: i64, to: i64) -> i64 {
        self.events_since(provider, from)
            .filter(|r| r.ts < to)
            .map(|r| r.w)
            .sum()
    }

    pub fn stats(&self, provider: &str, now: i64) -> TokenStats {
        let today = self
            .daily
            .get(provider)
            .and_then(|d| d.get(&day_key(now)))
            .cloned()
            .unwrap_or_default();
        let week_from = now - 7 * DAY;
        let mut models: BTreeMap<String, Breakdown> = BTreeMap::new();
        let mut projects: BTreeMap<String, Breakdown> = BTreeMap::new();
        let mut heatmap = vec![vec![0i64; 24]; 7];
        let today_date = local_date(now);
        let (mut week, mut cost_week, mut unpriced, mut last5h, mut cost5h) = (0, 0.0, 0, 0, 0.0);
        for r in self.events_since(provider, week_from) {
            week += r.w;
            cost_week += r.cost.unwrap_or(0.0);
            if r.cost.is_none() {
                unpriced += r.w;
            }
            if r.ts >= now - 5 * HOUR {
                last5h += r.w;
                cost5h += r.cost.unwrap_or(0.0);
            }
            let add = |map: &mut BTreeMap<String, Breakdown>, name: &str| {
                let b = map.entry(name.to_string()).or_insert_with(|| Breakdown {
                    name: name.to_string(),
                    ..Default::default()
                });
                b.w += r.w;
                b.cost += r.cost.unwrap_or(0.0);
            };
            add(&mut models, r.model.as_deref().unwrap_or("(알 수 없음)"));
            if let Some(p) = &r.project {
                add(&mut projects, p);
            }
            if let (Some(t), Some(d)) = (today_date, local_date(r.ts)) {
                let ago = (t - d).num_days();
                if (0..7).contains(&ago) {
                    heatmap[(6 - ago) as usize][local_hour(r.ts) as usize] += r.w;
                }
            }
        }
        let top = |m: BTreeMap<String, Breakdown>| {
            let mut v: Vec<Breakdown> = m.into_values().collect();
            v.sort_by_key(|b| std::cmp::Reverse(b.w));
            v.truncate(8);
            v
        };
        let cost30d = self.history(provider, 30, now).iter().map(|d| d.cost).sum();
        TokenStats {
            today: today.w,
            cost_today: today.cost,
            today_detail: today,
            last5h,
            week,
            cost5h,
            cost_week,
            cost30d,
            unpriced_week: unpriced,
            last_event_at: self.last_event_at.get(provider).copied().filter(|t| *t > 0),
            models: top(models),
            projects: top(projects),
            heatmap,
        }
    }

    /// Daily totals for the last `days` days, oldest first.
    pub fn history(&self, provider: &str, days: i64, now: i64) -> Vec<HistoryDay> {
        let by_day = self.daily.get(provider);
        (0..days)
            .rev()
            .map(|i| {
                let k = day_key(now - i * DAY);
                let d = by_day.and_then(|d| d.get(&k));
                HistoryDay {
                    day: k,
                    w: d.map(|d| d.w).unwrap_or(0),
                    cost: d.map(|d| d.cost).unwrap_or(0.0),
                }
            })
            .collect()
    }

    /// All stored days (for CSV export), oldest first.
    pub fn all_days(&self, provider: &str) -> Vec<(String, DayTotals)> {
        self.daily
            .get(provider)
            .map(|d| d.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default()
    }

    pub fn to_data(&self) -> LedgerData {
        LedgerData {
            v: LEDGER_VERSION,
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

    /// Returns false when the data is from an incompatible version (it is then ignored).
    pub fn load(&mut self, data: LedgerData) -> bool {
        if data.v != LEDGER_VERSION {
            return false;
        }
        self.events = data
            .events
            .into_iter()
            .map(|(p, v)| (p, v.into_iter().collect()))
            .collect();
        self.daily = data.daily.into_iter().collect();
        self.last_event_at = data.last_event_at.into_iter().collect();
        self.pending = data.pending;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deltas_cost_and_breakdowns() {
        let mut l = Ledger::new(8, 1000);
        let now = 10 * DAY;
        let t = |i: f64| Tokens {
            input: i,
            ..Default::default()
        };
        assert_eq!(
            l.record("x", "k", 500, t(100.0), None, None, now),
            0,
            "outside lookback"
        );
        let ts = now - HOUR;
        assert_eq!(
            l.record(
                "claude",
                "k2",
                ts,
                t(1_000_000.0),
                Some("claude-opus-5-5"),
                Some("app"),
                now
            ),
            1_000_000
        );
        assert_eq!(
            l.record(
                "claude",
                "k2",
                ts,
                t(1_000_000.0),
                Some("claude-opus-5-5"),
                Some("app"),
                now
            ),
            0,
            "duplicate"
        );
        assert_eq!(
            l.record(
                "claude",
                "k3",
                ts,
                t(500_000.0),
                Some("mystery-model"),
                Some("lib"),
                now
            ),
            500_000
        );
        let s = l.stats("claude", now);
        assert_eq!(s.week, 1_500_000);
        assert!(
            (s.cost_week - 4.0).abs() < 1e-9,
            "only the priced model counts: {}",
            s.cost_week
        );
        assert_eq!(s.unpriced_week, 500_000);
        assert_eq!(s.projects[0].name, "app");
        assert_eq!(s.models.len(), 2);
        assert_eq!(s.heatmap.len(), 7);
        assert_eq!(
            s.heatmap[6].iter().sum::<i64>() + s.heatmap[5].iter().sum::<i64>(),
            1_500_000
        );
        assert_eq!(l.take_pending().get("claude"), Some(&1_500_000));
        assert_eq!(
            weigh(&Tokens {
                input: 1000.0,
                output: 500.0,
                cache_read: 10000.0,
                cache_write: 2000.0
            }),
            4500
        );

        // re-recording with new numbers moves the daily totals by the difference
        l.record(
            "claude",
            "k2",
            ts,
            t(2_000_000.0),
            Some("claude-opus-5-5"),
            None,
            now,
        );
        let h = l.history("claude", 1, now);
        assert_eq!(h[0].w + l.history("claude", 2, now)[0].w, 2_500_000);
    }
}
