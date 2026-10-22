//! Time helpers: lenient parsing of LLM-written timestamps and short "ago" labels.

use chrono::{DateTime, Local, NaiveDateTime, TimeZone};

const NAIVE_FORMATS: [&str; 3] = ["%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M"];

/// RFC 3339 first, then naive local formats (LLMs often drop the offset).
///
/// ```text
/// parse_loose("2026-10-22 10:55:12") -> Some(local 10:55:12)
/// ```
pub fn parse_loose(raw: &str) -> Option<DateTime<Local>> {
    let raw = raw.trim();
    if let Ok(stamped) = DateTime::parse_from_rfc3339(raw) {
        return Some(stamped.with_timezone(&Local));
    }
    NAIVE_FORMATS
        .iter()
        .find_map(|fmt| NaiveDateTime::parse_from_str(raw, fmt).ok())
        .and_then(|naive| Local.from_local_datetime(&naive).earliest())
}

/// Compact elapsed label: `12s`, `5m`, `3h`, `2d`. Future times read as `now`.
pub fn ago(then: DateTime<Local>, now: DateTime<Local>) -> String {
    let secs = (now - then).num_seconds();
    match secs {
        s if s < 5 => "now".to_string(),
        s if s < 60 => format!("{s}s"),
        s if s < 3600 => format!("{}m", s / 60),
        s if s < 86_400 => format!("{}h", s / 3600),
        s => format!("{}d", s / 86_400),
    }
}

/// Sentence form of [`ago`]: `just now`, `5m ago`.
pub fn ago_phrase(then: DateTime<Local>, now: DateTime<Local>) -> String {
    match ago(then, now).as_str() {
        "now" => "just now".to_string(),
        short => format!("{short} ago"),
    }
}

/// `14200` -> `14,200`.
pub fn group_thousands(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Timelike};

    #[test]
    fn parses_rfc3339_and_naive() {
        assert!(parse_loose("2026-10-22T10:55:12-03:00").is_some());
        let naive = parse_loose("2026-10-22 10:55:12").unwrap();
        assert_eq!((naive.hour(), naive.minute()), (10, 55));
        assert!(parse_loose("2026-10-22T10:55:12").is_some());
        assert!(parse_loose("yesterday").is_none());
    }

    #[test]
    fn ago_buckets() {
        let now = Local::now();
        assert_eq!(ago(now, now), "now");
        assert_eq!(ago(now - Duration::seconds(30), now), "30s");
        assert_eq!(ago(now - Duration::minutes(5), now), "5m");
        assert_eq!(ago(now - Duration::hours(3), now), "3h");
        assert_eq!(ago(now - Duration::days(2), now), "2d");
        assert_eq!(ago(now + Duration::hours(1), now), "now");
        assert_eq!(ago_phrase(now, now), "just now");
        assert_eq!(ago_phrase(now - Duration::minutes(5), now), "5m ago");
    }

    #[test]
    fn groups_thousands() {
        assert_eq!(group_thousands(0), "0");
        assert_eq!(group_thousands(999), "999");
        assert_eq!(group_thousands(14200), "14,200");
        assert_eq!(group_thousands(1234567), "1,234,567");
    }
}
