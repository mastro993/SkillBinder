//! Why each skill changed on this device since the last commit Push or Sync made (Design 3): one
//! row per slug, written by the change that made it, which a commit reads into its note and then
//! clears.

use crate::{lifecycle::format_timestamp, schema::pending_changes, storage};
use diesel::{prelude::*, upsert::excluded};
use skillbinder_proto::{AppResult, Reason};
use std::collections::BTreeMap;

/// Records `reason` as why `slug` changed, replacing any earlier one.
pub(crate) fn record(
    connection: &mut SqliteConnection,
    slug: &str,
    reason: &Reason,
) -> AppResult<()> {
    insert(connection, slug, reason, true)
}

/// Records `reason` as why `slug` changed unless an earlier change already says why.
pub(crate) fn record_unless_known(
    connection: &mut SqliteConnection,
    slug: &str,
    reason: &Reason,
) -> AppResult<()> {
    insert(connection, slug, reason, false)
}

/// Why each skill with a row changed, by slug.
pub(crate) fn reasons(connection: &mut SqliteConnection) -> AppResult<BTreeMap<String, Reason>> {
    let rows = pending_changes::table
        .select((
            pending_changes::slug,
            pending_changes::reason,
            pending_changes::detail,
        ))
        .load::<(String, String, Option<String>)>(connection)
        .map_err(storage::error("reading why skills changed"))?;
    Ok(rows
        .into_iter()
        .map(|(slug, word, detail)| (slug, Reason::stored(&word, detail)))
        .collect())
}

/// Clears every row once a commit holds the library's managed files as they are, which leaves
/// no change for any row to explain.
pub(crate) fn clear(connection: &mut SqliteConnection) -> AppResult<()> {
    diesel::delete(pending_changes::table)
        .execute(connection)
        .map(drop)
        .map_err(storage::error("clearing why skills changed"))
}

fn insert(
    connection: &mut SqliteConnection,
    slug: &str,
    reason: &Reason,
    replace: bool,
) -> AppResult<()> {
    let insert = diesel::insert_into(pending_changes::table).values((
        pending_changes::slug.eq(slug),
        pending_changes::reason.eq(reason.word()),
        pending_changes::detail.eq(reason.detail()),
        pending_changes::at.eq(format_timestamp()),
    ));
    let inserted = if replace {
        insert
            .on_conflict(pending_changes::slug)
            .do_update()
            .set((
                pending_changes::reason.eq(excluded(pending_changes::reason)),
                pending_changes::detail.eq(excluded(pending_changes::detail)),
                pending_changes::at.eq(excluded(pending_changes::at)),
            ))
            .execute(connection)
    } else {
        insert.on_conflict_do_nothing().execute(connection)
    };
    inserted
        .map(drop)
        .map_err(storage::error("recording why a skill changed"))
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "Test assertions report failures.")]
mod tests {
    use super::{clear, reasons, record, record_unless_known};
    use skillbinder_proto::Reason;
    use std::collections::BTreeMap;

    /// The migration adds the table to an existing database, and a later change replaces an
    /// earlier one's reason unless it only wrote a record.
    #[test]
    fn a_change_records_why_until_a_commit_clears_it() {
        let root = tempfile::tempdir().unwrap();
        let mut connection = crate::database::open(&root.path().join("state.db")).unwrap();
        let imported = Reason::Imported {
            from: Some("Claude Code".into()),
        };
        record(&mut connection, "review", &imported).unwrap();
        record_unless_known(&mut connection, "review", &Reason::Record).unwrap();
        record_unless_known(&mut connection, "notes", &Reason::Record).unwrap();
        assert_eq!(
            reasons(&mut connection).unwrap(),
            BTreeMap::from([
                ("notes".to_owned(), Reason::Record),
                ("review".to_owned(), imported),
            ])
        );
        record(&mut connection, "review", &Reason::Updated).unwrap();
        assert_eq!(
            reasons(&mut connection).unwrap().get("review"),
            Some(&Reason::Updated)
        );
        clear(&mut connection).unwrap();
        assert!(reasons(&mut connection).unwrap().is_empty());
    }
}
