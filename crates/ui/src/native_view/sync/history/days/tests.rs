use super::*;
use chrono::FixedOffset;
use skillbinder_proto::ChangeKind;

/// One hour east of UTC, so a UTC evening can fall on the next local day.
fn zone() -> FixedOffset {
    FixedOffset::east_opt(3_600).unwrap_or_else(|| unreachable!())
}

/// A local time in [`zone`].
fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<FixedOffset> {
    zone()
        .with_ymd_and_hms(year, month, day, hour, minute, 0)
        .single()
        .unwrap_or_else(|| unreachable!())
}

/// Friday, November 27, 2026, at 10:00.
fn now() -> DateTime<FixedOffset> {
    at(2026, 11, 27, 10, 0)
}

fn change(commit: &str, when: DateTime<FixedOffset>, slugs: &[&str]) -> SyncChange {
    SyncChange {
        commit: commit.to_owned(),
        at: u64::try_from(when.timestamp()).unwrap_or_default(),
        device: None,
        skills: slugs
            .iter()
            .map(|slug| ChangedSkill {
                slug: (*slug).to_owned(),
                kind: ChangeKind::Changed,
                reason: None,
            })
            .collect(),
    }
}

fn grouped(changes: &[SyncChange]) -> Vec<(String, Vec<&str>)> {
    days(changes, &now())
        .into_iter()
        .map(|day| {
            let commits = day.changes.iter().map(|c| c.commit.as_str()).collect();
            (day.heading, commits)
        })
        .collect()
}

#[test]
fn changes_group_under_today_yesterday_then_their_date() {
    let changes = [
        change("a", at(2026, 11, 27, 9, 15), &[]),
        change("b", at(2026, 11, 27, 0, 0), &[]),
        change("c", at(2026, 11, 26, 23, 59), &[]),
        change("d", at(2026, 11, 25, 18, 2), &[]),
        change("e", at(2026, 11, 12, 8, 0), &[]),
    ];
    assert_eq!(
        grouped(&changes),
        [
            ("Today".to_owned(), vec!["a", "b"]),
            ("Yesterday".to_owned(), vec!["c"]),
            ("Wed, Nov 25".to_owned(), vec!["d"]),
            ("Thu, Nov 12".to_owned(), vec!["e"]),
        ]
    );
    assert_eq!(grouped(&[]), []);
}

#[test]
fn days_and_times_follow_the_local_zone_across_midnight_and_years() {
    // 23:30 UTC on the 26th is already 00:30 on the 27th an hour east.
    let utc_evening = change("a", at(2026, 11, 27, 0, 30), &[]);
    assert_eq!(time(&utc_evening, &now()), "00:30");
    assert_eq!(grouped(&[utc_evening])[0].0, "Today");
    let new_year = |commit, when| grouped(&[change(commit, when, &[])])[0].0.clone();
    assert_eq!(new_year("b", at(2025, 12, 31, 23, 0)), "Wed, Dec 31, 2025");
    let january = at(2027, 1, 1, 8, 0);
    let yesterday = change("c", at(2026, 12, 31, 22, 0), &[]);
    assert_eq!(days(&[yesterday], &january)[0].heading, "Yesterday");
}

#[test]
fn a_change_shows_four_chips_then_how_many_more() {
    let six = change("a", now(), &["a", "b", "c", "d", "e", "f"]);
    let (shown, more) = chips(&six.skills);
    assert_eq!(shown.len(), 4);
    assert_eq!(more, 2);
    let three = change("b", now(), &["a", "b", "c"]);
    assert_eq!(chips(&three.skills).1, 0);
}

#[test]
fn a_skill_s_newest_change_is_the_first_listed() {
    let changes = [
        change("new", now(), &["review"]),
        change("old", at(2026, 11, 20, 9, 0), &["review", "deploy"]),
    ];
    let newest = newest(&changes);
    assert_eq!(newest.get("review"), Some(&"new"));
    assert_eq!(newest.get("deploy"), Some(&"old"));
}
