//! Ignored skills (ADR 0033): found skills the user chose never to be offered, by normalized
//! slug, on this device only. An ignore only filters what is offered; scans still find the skill,
//! plans still prepare it, and its files are never touched.

use crate::{
    database,
    library::normalized,
    schema::{ignored_skills, skills},
    storage,
    worker::{Action, Engine, State},
};
use diesel::prelude::*;
use skillbinder_proto::*;
use std::collections::BTreeSet;

impl Engine {
    /// Ignores the found skills `slugs`, or stops ignoring them when `ignored` is false; the next
    /// snapshot lists the ignored keys. Nothing is scanned, planned, or written outside the
    /// database, and nothing changes unless every slug is accepted.
    pub async fn set_ignored(&self, slugs: Vec<String>, ignored: bool) -> AppResult<()> {
        let subject = vec![("slugs", slugs.join(",")), ("ignored", ignored.to_string())];
        self.action(Action::SetIgnored, subject, move |engine| async move {
            engine
                .call(move |state| state.set_ignored(&slugs, ignored))
                .await
        })
        .await
    }
}

impl State {
    /// Ignores `slugs`, each one the latest completed scan found and the library does not hold,
    /// or stops ignoring them, each one ignored or found; an ignored slug stays as it was ignored.
    pub(crate) fn set_ignored(&mut self, slugs: &[String], ignored: bool) -> AppResult<()> {
        let found = self.found_keys()?;
        let held = self.held_keys()?;
        let kept = self.ignored_keys()?;
        for slug in slugs {
            let key = normalized(slug);
            if ignored && held.contains(&key) {
                return Err(AppError::validation(format!("{slug} is in your library.")));
            }
            if ignored && !found.contains(&key) {
                return Err(AppError::not_found(format!("No found skill named {slug}.")));
            }
            if !ignored && !kept.contains(&key) && !found.contains(&key) {
                return Err(AppError::not_found(format!(
                    "No ignored skill named {slug}."
                )));
            }
        }
        let now = chrono::Utc::now().timestamp();
        database::transaction(&mut self.database.borrow_mut(), |connection| {
            for slug in slugs {
                let key = normalized(slug);
                if ignored {
                    diesel::insert_into(ignored_skills::table)
                        .values((
                            ignored_skills::key.eq(&key),
                            ignored_skills::slug.eq(slug),
                            ignored_skills::ignored_at.eq(now),
                        ))
                        .on_conflict_do_nothing()
                        .execute(connection)
                        .map_err(storage::error("ignoring a found skill"))?;
                } else {
                    forget(connection, &key)?;
                }
            }
            Ok(())
        })
    }

    /// Every ignored key, including ones no scan finds now.
    pub(crate) fn ignored_keys(&self) -> AppResult<BTreeSet<String>> {
        ignored_skills::table
            .select(ignored_skills::key)
            .load::<String>(&mut *self.database.borrow_mut())
            .map(BTreeSet::from_iter)
            .map_err(storage::error("reading the ignored skills"))
    }

    /// The keys of the found skills the latest completed scan found outside the library.
    fn found_keys(&self) -> AppResult<BTreeSet<String>> {
        let (scan, _) = self.scans.current()?;
        Ok(scan
            .map(|scan| scan.candidates)
            .unwrap_or_default()
            .into_iter()
            .filter(|candidate| candidate.in_library().is_none())
            .map(|candidate| candidate.key)
            .collect())
    }

    /// The keys of the skills the library holds now.
    fn held_keys(&self) -> AppResult<BTreeSet<String>> {
        skills::table
            .select(skills::slug)
            .load::<String>(&mut *self.database.borrow_mut())
            .map(|slugs| slugs.iter().map(|slug| normalized(slug)).collect())
            .map_err(storage::error("reading the library's slugs"))
    }
}

/// Stops ignoring `key`, as entering the library does in the transaction that adds the skill.
pub(crate) fn forget(connection: &mut SqliteConnection, key: &str) -> AppResult<()> {
    diesel::delete(ignored_skills::table.find(key))
        .execute(connection)
        .map(drop)
        .map_err(storage::error("stopping ignoring a skill"))
}
