//! History's words and grouping (Y6): changes by day under "Today", "Yesterday", or a date, each
//! at its time, with at most four skill chips, and which change made each skill's current
//! version.

use super::super::super::library_dates::from_unix;
use chrono::{DateTime, Datelike, TimeZone};
use skillbinder_proto::{ChangedSkill, SyncChange};
use std::collections::BTreeMap;
use std::fmt::Display;

/// The most skill chips a change shows before "+<n> more".
const CHIPS: usize = 4;

/// One day of History: its heading, over its changes, newest first.
#[derive(Debug, Eq, PartialEq)]
pub(super) struct Day<'a> {
    pub(super) heading: String,
    pub(super) changes: Vec<&'a SyncChange>,
}

/// `changes`, newest first, grouped by their day in `now`'s time zone.
pub(super) fn days<'a, Tz: TimeZone>(changes: &'a [SyncChange], now: &DateTime<Tz>) -> Vec<Day<'a>>
where
    Tz::Offset: Display,
{
    let mut days: Vec<(chrono::NaiveDate, Day<'a>)> = Vec::new();
    for change in changes {
        let date = local(change, now).date_naive();
        match days.last_mut() {
            Some((last, day)) if *last == date => day.changes.push(change),
            _ => days.push((
                date,
                Day {
                    heading: heading(date, now.date_naive()),
                    changes: vec![change],
                },
            )),
        }
    }
    days.into_iter().map(|(_, day)| day).collect()
}

/// "Today", "Yesterday", "Wed, Nov 12", or, in another year, "Tue, Nov 12, 2024".
fn heading(date: chrono::NaiveDate, today: chrono::NaiveDate) -> String {
    if date == today {
        "Today".to_owned()
    } else if today.pred_opt() == Some(date) {
        "Yesterday".to_owned()
    } else if date.year() == today.year() {
        date.format("%a, %b %-d").to_string()
    } else {
        date.format("%a, %b %-d, %Y").to_string()
    }
}

/// When `change` was made, as "09:15" in `now`'s time zone.
pub(super) fn time<Tz: TimeZone>(change: &SyncChange, now: &DateTime<Tz>) -> String
where
    Tz::Offset: Display,
{
    local(change, now).format("%H:%M").to_string()
}

fn local<Tz: TimeZone>(change: &SyncChange, now: &DateTime<Tz>) -> DateTime<Tz> {
    from_unix(change.at)
        .unwrap_or_default()
        .with_timezone(&now.timezone())
}

/// The skills a change shows as chips, and how many more it holds.
pub(super) fn chips(skills: &[ChangedSkill]) -> (&[ChangedSkill], usize) {
    let shown = &skills[..skills.len().min(CHIPS)];
    (shown, skills.len() - shown.len())
}

/// Each slug's newest change among `changes`, newest first, by commit: the change whose version
/// of the skill the library holds now, unless that change removed it.
pub(super) fn newest(changes: &[SyncChange]) -> BTreeMap<&str, &str> {
    let mut newest = BTreeMap::new();
    for change in changes {
        for skill in &change.skills {
            newest
                .entry(skill.slug.as_str())
                .or_insert(change.commit.as_str());
        }
    }
    newest
}

#[cfg(test)]
mod tests;
