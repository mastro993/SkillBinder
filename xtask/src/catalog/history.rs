//! The published `history.json`: each night's all-time installs by `<repository>#<path>`, for
//! the last 30 nights, from which trending is each skill's gain over the last 7 days.

use super::date::Day;
use anyhow::Context as _;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(super) const NIGHTS: i64 = 30;
pub(super) const WEEK: i64 = 7;

pub(super) type Totals = BTreeMap<String, u64>;

#[derive(Default, Deserialize, Serialize)]
pub(super) struct History(BTreeMap<String, Totals>);

/// A skill's key in the history.
pub(super) fn key(repository: &str, path: &str) -> String {
    format!("{repository}#{path}")
}

impl History {
    pub(super) fn parse(bytes: &[u8]) -> anyhow::Result<Self> {
        let history: Self = serde_json::from_slice(bytes).context("history.json is malformed")?;
        for date in history.0.keys() {
            Day::parse(date).context("history.json has an invalid night")?;
        }
        Ok(history)
    }

    pub(super) fn nights(&self) -> usize {
        self.0.len()
    }

    /// Records `today`'s totals, replacing an earlier run's on the same day, and drops nights
    /// that are 30 or more days old.
    pub(super) fn record(&mut self, today: Day, totals: Totals) {
        let oldest = today.minus(NIGHTS - 1).to_string();
        self.0.insert(today.to_string(), totals);
        self.0.retain(|date, _| *date >= oldest);
    }

    /// `key`'s gain from the oldest night within the last 7 days that counts it to `today`, or
    /// 0 when no earlier night counts it.
    pub(super) fn gain(&self, today: Day, key: &str) -> u64 {
        let (from, to) = (today.minus(WEEK).to_string(), today.to_string());
        let mut nights = self
            .0
            .range(from..=to)
            .filter_map(|(_, totals)| totals.get(key));
        let first = nights.next().copied().unwrap_or_default();
        let last = nights.next_back().copied().unwrap_or(first);
        last.saturating_sub(first)
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::expect_used,
        reason = "Test assertions report fixture failures."
    )]
    use super::{Day, History, Totals};

    fn day(text: &str) -> Day {
        Day::parse(text).expect("valid date")
    }

    fn totals(installs: u64) -> Totals {
        Totals::from([("r#s".to_owned(), installs)])
    }

    #[test]
    fn a_run_adds_one_night_and_keeps_thirty() {
        let mut history = History::default();
        let start = day("2026-10-01");
        for night in 0..40 {
            history.record(start.minus(-night), totals(10));
        }
        assert_eq!(history.nights(), 30);
        history.record(start.minus(-39), totals(11));
        assert_eq!(
            history.nights(),
            30,
            "a second run the same day replaces the night"
        );
    }

    #[test]
    fn trending_is_the_gain_over_the_last_seven_days() {
        let mut history = History::default();
        let today = day("2026-11-19");
        for (ago, installs) in [(9, 1), (7, 100), (3, 140), (0, 170)] {
            history.record(today.minus(ago), totals(installs));
        }
        assert_eq!(history.gain(today, "r#s"), 70);
        assert_eq!(history.gain(today, "r#unknown"), 0);
    }

    #[test]
    fn short_history_counts_from_its_first_night() {
        let mut history = History::default();
        let today = day("2026-11-19");
        history.record(today.minus(2), totals(40));
        history.record(today, totals(45));
        assert_eq!(history.gain(today, "r#s"), 5);
        let mut fresh = History::default();
        fresh.record(today, totals(45));
        assert_eq!(fresh.gain(today, "r#s"), 0, "the first night has no gain");
    }

    #[test]
    fn a_falling_count_gains_nothing() {
        let mut history = History::default();
        let today = day("2026-11-19");
        history.record(today.minus(1), totals(50));
        history.record(today, totals(20));
        assert_eq!(history.gain(today, "r#s"), 0);
    }

    #[test]
    fn invalid_history_is_refused() {
        assert!(History::parse(br#"{"2026-11-19":{"r#s":1}}"#).is_ok());
        for text in [r#"{"yesterday":{}}"#, r#"{"2026-11-19":{"r#s":-1}}"#, "[]"] {
            assert!(History::parse(text.as_bytes()).is_err(), "accepted {text}");
        }
    }
}
