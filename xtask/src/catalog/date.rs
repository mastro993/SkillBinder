//! UTC calendar days and timestamps, without a date crate: the job needs only day arithmetic,
//! `YYYY-MM-DD` keys, and one RFC 3339 timestamp.

use anyhow::Context as _;
use std::{fmt, time::SystemTime};

const SECONDS_PER_DAY: u64 = 86_400;

/// A UTC calendar day, counted from 1970-01-01.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct Day(i64);

impl Day {
    pub(super) fn of(now: SystemTime) -> anyhow::Result<Self> {
        let seconds = now.duration_since(SystemTime::UNIX_EPOCH)?.as_secs();
        Ok(Self(i64::try_from(seconds / SECONDS_PER_DAY)?))
    }

    pub(super) fn minus(self, days: i64) -> Self {
        Self(self.0 - days)
    }

    /// Parses `YYYY-MM-DD`, refusing any day the calendar lacks.
    pub(super) fn parse(text: &str) -> anyhow::Result<Self> {
        let invalid = || format!("{text} is not a YYYY-MM-DD date");
        let mut parts = text.splitn(3, '-');
        let mut next = |width: usize| {
            parts
                .next()
                .filter(|part| part.len() == width && part.bytes().all(|b| b.is_ascii_digit()))
                .and_then(|part| part.parse::<i64>().ok())
                .with_context(invalid)
        };
        let (year, month, day) = (next(4)?, next(2)?, next(2)?);
        let parsed = Self(days_from_civil(year, month, day));
        anyhow::ensure!(parsed.civil() == (year, month, day), invalid());
        Ok(parsed)
    }

    /// Howard Hinnant's `civil_from_days`.
    fn civil(self) -> (i64, i64, i64) {
        let z = self.0 + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = doy - (153 * mp + 2) / 5 + 1;
        let month = if mp < 10 { mp + 3 } else { mp - 9 };
        let year = yoe + era * 400 + i64::from(month <= 2);
        (year, month, day)
    }
}

/// Howard Hinnant's `days_from_civil`.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year.rem_euclid(400);
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

impl fmt::Display for Day {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (year, month, day) = self.civil();
        write!(f, "{year:04}-{month:02}-{day:02}")
    }
}

/// `now` as an RFC 3339 UTC timestamp to the second.
pub(super) fn timestamp(now: SystemTime) -> anyhow::Result<String> {
    let seconds = now.duration_since(SystemTime::UNIX_EPOCH)?.as_secs();
    let time = seconds % SECONDS_PER_DAY;
    Ok(format!(
        "{}T{:02}:{:02}:{:02}Z",
        Day::of(now)?,
        time / 3_600,
        time % 3_600 / 60,
        time % 60
    ))
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::expect_used,
        reason = "Test assertions report fixture failures."
    )]
    use super::{Day, timestamp};
    use std::time::{Duration, SystemTime};

    #[test]
    fn days_round_trip_through_the_calendar() {
        for text in ["1970-01-01", "2000-02-29", "2026-11-19", "2028-12-31"] {
            assert_eq!(Day::parse(text).expect("valid date").to_string(), text);
        }
        let day = Day::parse("2026-03-01").expect("valid date");
        assert_eq!(day.minus(1).to_string(), "2026-02-28");
        assert_eq!(day.minus(30).to_string(), "2026-01-30");
    }

    #[test]
    fn impossible_or_malformed_dates_are_refused() {
        for text in [
            "2026-02-29",
            "2026-13-01",
            "2026-1-01",
            "26-01-01",
            "2026-01-01x",
            "",
        ] {
            assert!(Day::parse(text).is_err(), "{text} parsed");
        }
    }

    #[test]
    fn timestamps_are_utc_seconds() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_795_576_364);
        assert_eq!(timestamp(now).expect("after epoch"), "2026-11-25T03:12:44Z");
    }
}
