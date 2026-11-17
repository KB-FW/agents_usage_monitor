use chrono::{DateTime, Datelike, Local, NaiveDateTime, TimeZone};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

pub const MIN: i64 = 60 * 1000;
pub const HOUR: i64 = 60 * MIN;
pub const DAY: i64 = 24 * HOUR;

/// Milliseconds since the Unix epoch.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Local calendar day `YYYY-MM-DD` for a timestamp.
pub fn day_key(ts: i64) -> String {
    match Local.timestamp_millis_opt(ts).earliest() {
        Some(d) => format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day()),
        None => "1970-01-01".to_string(),
    }
}

/// Parse an ISO-8601 string or a seconds/milliseconds epoch number.
pub fn parse_time(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n.as_f64().and_then(epoch_to_ms),
        Value::String(s) => parse_time_str(s),
        _ => None,
    }
}

pub fn parse_time_str(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Some(d.timestamp_millis());
    }
    for fmt in [
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
    ] {
        if let Ok(n) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(n.and_utc().timestamp_millis());
        }
    }
    s.parse::<f64>().ok().and_then(epoch_to_ms)
}

fn epoch_to_ms(x: f64) -> Option<i64> {
    if !x.is_finite() || x <= 0.0 {
        return None;
    }
    Some(if x < 1e12 {
        (x * 1000.0) as i64
    } else {
        x as i64
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_times() {
        assert_eq!(
            parse_time(&json!("2026-11-17T09:00:00Z")),
            Some(1_794_906_000_000)
        );
        assert_eq!(
            parse_time(&json!("2026-11-17T09:00:00.500Z")),
            Some(1_794_906_000_500)
        );
        assert_eq!(parse_time(&json!(1_794_906_000)), Some(1_794_906_000_000));
        assert_eq!(
            parse_time(&json!(1_794_906_000_000i64)),
            Some(1_794_906_000_000)
        );
        assert_eq!(parse_time(&json!(null)), None);
        assert_eq!(parse_time(&json!("")), None);
    }
}
