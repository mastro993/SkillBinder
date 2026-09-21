//! The registered project-search roots.

use crate::schema::scan_roots;
use diesel::SqliteConnection;
use diesel::prelude::*;
use skillbinder_core::discovery::roots::ScanRoot;
use std::path::PathBuf;

/// Reported when a new root's canonical path is already covered.
pub(crate) const DUPLICATE_PATH_MESSAGE: &str = "a project-search root already covers that path";

/// One `scan_roots` row in table order: id, canonical path, display path, label, enabled, created.
type RootRow = (String, String, String, String, i32, i64);

/// The columns an update may change. A missing label leaves the stored one alone; the enabled
/// flag is stored as `INTEGER`, like the DDL says.
#[derive(AsChangeset)]
#[diesel(table_name = scan_roots)]
struct RootUpdate {
    label: Option<String>,
    enabled: i32,
}

fn from_row(row: RootRow) -> ScanRoot {
    let (id, canonical_path, display_path, label, enabled, created_at) = row;
    ScanRoot {
        id,
        canonical_path: PathBuf::from(canonical_path),
        display_path,
        label,
        enabled: enabled != 0,
        created_at: created_at as u64,
    }
}

pub(crate) fn insert(connection: &mut SqliteConnection, root: &ScanRoot) -> QueryResult<()> {
    diesel::insert_into(scan_roots::table)
        .values((
            scan_roots::id.eq(&root.id),
            scan_roots::canonical_path.eq(root.canonical_path.to_string_lossy().as_ref()),
            scan_roots::display_path.eq(&root.display_path),
            scan_roots::label.eq(&root.label),
            scan_roots::enabled.eq(root.enabled as i32),
            scan_roots::created_at.eq(root.created_at as i64),
        ))
        .execute(connection)?;
    Ok(())
}

/// Enabled roots first, then oldest first, ties broken by id.
pub(crate) fn list(connection: &mut SqliteConnection) -> QueryResult<Vec<ScanRoot>> {
    let rows: Vec<RootRow> = scan_roots::table
        .select(scan_roots::all_columns)
        .order((
            scan_roots::enabled.desc(),
            scan_roots::created_at.asc(),
            scan_roots::id.asc(),
        ))
        .load(connection)?;
    Ok(rows.into_iter().map(from_row).collect())
}

pub(crate) fn update(
    connection: &mut SqliteConnection,
    id: &str,
    label: Option<&str>,
    enabled: bool,
) -> QueryResult<Option<ScanRoot>> {
    let changed = diesel::update(scan_roots::table.filter(scan_roots::id.eq(id)))
        .set(RootUpdate {
            label: label.map(str::to_owned),
            enabled: enabled as i32,
        })
        .execute(connection)?;
    if changed == 0 {
        return Ok(None);
    }
    find(connection, id)
}

/// `true` when a row was removed; removing a missing root is not an error.
pub(crate) fn remove(connection: &mut SqliteConnection, id: &str) -> QueryResult<bool> {
    let removed =
        diesel::delete(scan_roots::table.filter(scan_roots::id.eq(id))).execute(connection)?;
    Ok(removed > 0)
}

fn find(connection: &mut SqliteConnection, id: &str) -> QueryResult<Option<ScanRoot>> {
    let row: Option<RootRow> = scan_roots::table
        .select(scan_roots::all_columns)
        .filter(scan_roots::id.eq(id))
        .first(connection)
        .optional()?;
    Ok(row.map(from_row))
}
