//! The journal row of a Sync merge (spec Design 5): written before the library's files move to
//! the merge, and deleted once HEAD is the merge, so a launch after an interruption finishes it.

use crate::{git::Merge, logging::Area, logging::Level, schema::journals, storage, worker::State};
use diesel::{prelude::*, upsert::excluded};
use skillbinder_proto::AppResult;

/// The id of the merge's row; one Sync runs at a time.
pub(super) const MERGE: &str = "sync-merge";

impl State {
    /// Records `merge` before the library's files move to it.
    pub(crate) fn save_merge(&self, merge: &Merge) -> AppResult<()> {
        let body = serde_json::to_string(merge).map_err(storage::error("serializing a merge"))?;
        diesel::insert_into(journals::table)
            .values((journals::id.eq(MERGE), journals::body.eq(&body)))
            .on_conflict(journals::id)
            .do_update()
            .set(journals::body.eq(excluded(journals::body)))
            .execute(&mut *self.database.borrow_mut())
            .map(drop)
            .map_err(storage::error("saving a merge"))
    }

    /// Forgets the merge once HEAD is the merged commit.
    pub(crate) fn forget_merge(&self) -> AppResult<()> {
        diesel::delete(journals::table.find(MERGE))
            .execute(&mut *self.database.borrow_mut())
            .map(drop)
            .map_err(storage::error("removing a merge"))
    }

    /// Finishes a merge an interruption left, if any. Without Git it waits for one, and the
    /// library stays unchanged until a launch that finds Git finishes it; returns whether
    /// nothing is left to finish.
    pub(crate) fn recover_merge(&self) -> AppResult<bool> {
        let body = journals::table
            .find(MERGE)
            .select(journals::body)
            .first::<String>(&mut *self.database.borrow_mut())
            .optional()
            .map_err(storage::error("loading a merge"))?;
        let Some(body) = body else {
            return Ok(true);
        };
        let merge: Merge =
            serde_json::from_str(&body).map_err(storage::error("parsing a merge"))?;
        let Some(git) = &self.git else {
            self.log
                .event(Level::Warn, Area::Library, "sync recovery waits for git");
            return Ok(false);
        };
        git.finish_merge(&self.library_dir(), &merge)?;
        self.forget_merge()?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests;
