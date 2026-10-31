//! The managed copies and unapplied directories SkillBinder records, in machine-local SQLite.

use super::plan::{Issue, Row};
use crate::schema::managed_copies;
use diesel::{prelude::*, upsert::excluded};
use skillbinder_proto::{AppError, AppResult, WorkspaceId};
use std::collections::BTreeMap;

/// A global or workspace skill directory, as its resolved path, and a slug.
pub(crate) type Pair = (String, String);

/// Every recorded directory and slug.
pub(super) fn load(connection: &mut SqliteConnection) -> AppResult<BTreeMap<Pair, Row>> {
    managed_copies::table
        .select((
            managed_copies::directory,
            managed_copies::slug,
            managed_copies::digest,
            managed_copies::issue,
        ))
        .load::<(String, String, Option<String>, Option<String>)>(connection)
        .map_err(|_| AppError::storage())?
        .into_iter()
        .map(|(directory, slug, digest, issue)| {
            let issue = issue
                .as_deref()
                .map(serde_json::from_str::<Issue>)
                .transpose()
                .map_err(|_| AppError::storage())?;
            Ok(((directory, slug), Row { digest, issue }))
        })
        .collect()
}

/// Records each directory's new row, or forgets it when there is none.
pub(super) fn save(
    connection: &mut SqliteConnection,
    changes: &[(Pair, Option<Row>)],
) -> AppResult<()> {
    let storage = |_| AppError::storage();
    for ((directory, slug), row) in changes {
        let key = managed_copies::table.find((directory, slug));
        let Some(row) = row else {
            diesel::delete(key).execute(connection).map_err(storage)?;
            continue;
        };
        let issue = row
            .issue
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|_| AppError::storage())?;
        diesel::insert_into(managed_copies::table)
            .values((
                managed_copies::directory.eq(directory),
                managed_copies::slug.eq(slug),
                managed_copies::digest.eq(&row.digest),
                managed_copies::issue.eq(&issue),
            ))
            .on_conflict((managed_copies::directory, managed_copies::slug))
            .do_update()
            .set((
                managed_copies::digest.eq(excluded(managed_copies::digest)),
                managed_copies::issue.eq(excluded(managed_copies::issue)),
            ))
            .execute(connection)
            .map_err(storage)?;
    }
    Ok(())
}

/// Forgets `records`, leaving their copies in place, unmanaged.
pub(crate) fn forget(connection: &mut SqliteConnection, records: &[Pair]) -> AppResult<()> {
    let forgotten: Vec<_> = records.iter().map(|pair| (pair.clone(), None)).collect();
    save(connection, &forgotten)
}

/// The workspace a record is held for, and its workspace skill directory the record is held
/// through, such as `.claude/skills`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Holder {
    pub workspace: WorkspaceId,
    pub dir: String,
}

/// Who each record is held for, or `None` for your agents.
pub(super) fn owners(
    connection: &mut SqliteConnection,
) -> AppResult<BTreeMap<Pair, Option<Holder>>> {
    Ok(managed_copies::table
        .select((
            managed_copies::directory,
            managed_copies::slug,
            managed_copies::workspace_id,
            managed_copies::workspace_dir,
        ))
        .load::<(String, String, Option<String>, Option<String>)>(connection)
        .map_err(|_| AppError::storage())?
        .into_iter()
        .map(|(directory, slug, workspace, dir)| {
            let holder = workspace.map(|workspace| Holder {
                workspace: WorkspaceId(workspace),
                dir: dir.unwrap_or_default(),
            });
            ((directory, slug), holder)
        })
        .collect())
}

/// Records who each recorded pair is held for, or `None` for your agents.
pub(crate) fn claim(
    connection: &mut SqliteConnection,
    claims: &[(Pair, Option<Holder>)],
) -> AppResult<()> {
    for ((directory, slug), holder) in claims {
        diesel::update(managed_copies::table.find((directory, slug)))
            .set((
                managed_copies::workspace_id
                    .eq(holder.as_ref().map(|holder| holder.workspace.as_str())),
                managed_copies::workspace_dir.eq(holder.as_ref().map(|holder| &holder.dir)),
            ))
            .execute(connection)
            .map_err(|_| AppError::storage())?;
    }
    Ok(())
}
