//! When a Library skill was imported and when its files last changed, as relative times whose
//! tooltips show the exact local date and time.

use chrono::{DateTime, Local, Utc};

/// How long ago `then` was, in the largest whole unit: "just now", "5 minutes ago", "3 days ago".
pub(super) fn relative(then: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let seconds = (now - then).num_seconds().max(0);
    let (count, unit) = match seconds {
        0..60 => return "just now".to_owned(),
        60..3_600 => (seconds / 60, "minute"),
        3_600..86_400 => (seconds / 3_600, "hour"),
        86_400..2_592_000 => (seconds / 86_400, "day"),
        2_592_000..31_536_000 => (seconds / 2_592_000, "month"),
        _ => (seconds / 31_536_000, "year"),
    };
    let plural = if count == 1 { "" } else { "s" };
    format!("{count} {unit}{plural} ago")
}

/// The exact time in this device's time zone, such as "14 Oct 2026, 13:05".
pub(super) fn exact(time: DateTime<Utc>) -> String {
    time.with_timezone(&Local)
        .format("%-d %b %Y, %H:%M")
        .to_string()
}

/// A portable RFC 3339 import time.
pub(super) fn parse(time: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(time)
        .ok()
        .map(|time| time.with_timezone(&Utc))
}

/// A Unix time in seconds, as the engine reports file changes.
pub(super) fn from_unix(seconds: u64) -> Option<DateTime<Utc>> {
    DateTime::from_timestamp(i64::try_from(seconds).ok()?, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn relative_times_use_the_largest_whole_unit() {
        let now = DateTime::from_timestamp(1_800_000_000, 0).unwrap_or_default();
        let ago = |seconds| relative(now - Duration::seconds(seconds), now);
        assert_eq!(
            ago(-30),
            "just now",
            "a clock ahead of the file reads as now"
        );
        assert_eq!(ago(59), "just now");
        assert_eq!(ago(60), "1 minute ago");
        assert_eq!(ago(7_200), "2 hours ago");
        assert_eq!(ago(86_400 * 3), "3 days ago");
        assert_eq!(ago(2_592_000 * 2), "2 months ago");
        assert_eq!(ago(31_536_000), "1 year ago");
    }

    #[test]
    fn portable_and_engine_times_parse() {
        assert_eq!(
            parse("2026-10-14T12:00:00Z"),
            from_unix(1_791_979_200),
            "both describe the same instant"
        );
        assert_eq!(parse("yesterday"), None);
    }
}
