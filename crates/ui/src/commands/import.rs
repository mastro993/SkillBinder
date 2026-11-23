//! `scan` and `import`: Library's Scan for skills and its Import skills dialog (ADR 0008). Both
//! scan afresh and prepare one plan for every found skill not in library, as the dialog does.
//! `import` takes the skills the command names, or with `--all` the ones the dialog ticks, and
//! states the copies it replaces before anything changes (spec rule 7).

use super::run::{blocking, free, on_runtime};
use super::{Peer, resolve};
use crate::native_view::{
    Activity, Landing, Row, default_ticks, groups, location_label, replaced_in, rows, selection,
};
use gpui_kit::AsyncApp;
use skillbinder_client::Client;
use skillbinder_proto::command_line::{
    Changed, CommandError, ErrorCode, FoundRow, FoundVersion, Output, Permission,
};
use skillbinder_proto::{
    AppSnapshot, Candidate, CandidateId, FoundImport, MAX_IMPORT, ValidationStatus, Workspace,
};
use std::collections::BTreeSet;

/// How many characters of a version's digest the command shows and takes.
const SHORT: usize = 7;

/// `scan`: the found skills not in library, grouped as the dialog groups them.
pub(super) async fn scan(client: &Client) -> Result<Output, CommandError> {
    let found = prepare(client, Vec::new()).await?;
    Ok(Output::Found(found_rows(&found, &client.snapshot())))
}

/// Scans and prepares the plan, taking the `versions` chosen, as the dialog does.
async fn prepare(client: &Client, versions: Vec<CandidateId>) -> Result<FoundImport, CommandError> {
    let prepared = on_runtime(client, move |client| async move {
        client.prepare_found_import(versions).await
    });
    Ok(prepared.await?)
}

/// The dialog's rows of `found`, each with its versions and where they were found.
pub(super) fn found_rows(found: &FoundImport, snapshot: &AppSnapshot) -> Vec<FoundRow> {
    let groups = groups(found, &snapshot.workspaces);
    let ticked = &default_ticks(&groups);
    groups
        .iter()
        .flat_map(|group| {
            let place = group.place.name(&snapshot.workspaces);
            group.rows.iter().map(move |row| {
                let shown = row.shown();
                FoundRow {
                    slug: shown.map_or_else(|| row.key().to_owned(), |v| v.slug.clone()),
                    description: shown.map(|v| v.description.clone()).unwrap_or_default(),
                    place: place.clone(),
                    ticked: ticked.contains(row.key()),
                    versions: row
                        .skill
                        .versions
                        .iter()
                        .map(|version| found_version(version, row, snapshot))
                        .collect(),
                }
            })
        })
        .collect()
}

fn found_version(version: &Candidate, row: &Row<'_>, snapshot: &AppSnapshot) -> FoundVersion {
    let planned = row.item.filter(|item| item.candidate_id == version.id);
    FoundVersion {
        version: short(&version.id),
        planned: planned.is_some(),
        modified: version.modified,
        validation: version.validation,
        locations: version
            .locations
            .iter()
            .map(|location| location_label(location, snapshot))
            .collect(),
        replaces: planned.map_or_else(Vec::new, |item| {
            item.replaces
                .iter()
                .map(|directory| place(directory, snapshot))
                .collect()
        }),
    }
}

fn short(id: &CandidateId) -> String {
    id.0.chars().take(SHORT).collect()
}

/// How the review names the place of a copy an import replaces.
fn place(directory: &str, snapshot: &AppSnapshot) -> String {
    replaced_in(
        directory,
        &snapshot.agents,
        &snapshot.workspaces,
        &snapshot.home,
    )
}

/// The skills `import` takes, by key, and the versions it names that the plan does not take.
#[derive(Debug, Default, Eq, PartialEq)]
pub(super) struct Picked {
    keys: Vec<String>,
    versions: Vec<CandidateId>,
}

/// What `named` names among `found`, each `<slug>` or `<slug>@<version>`, or with `all` what the
/// dialog ticks. A skill with several versions needs one named.
pub(super) fn pick(
    found: &FoundImport,
    workspaces: &[Workspace],
    named: &[String],
    all: bool,
) -> Result<Picked, CommandError> {
    let groups = groups(found, workspaces);
    if all {
        let ticked = default_ticks(&groups);
        return Ok(Picked {
            keys: rows(&groups)
                .filter(|row| ticked.contains(row.key()))
                .map(|row| row.key().to_owned())
                .collect(),
            versions: Vec::new(),
        });
    }
    let listed: Vec<&Row<'_>> = rows(&groups).collect();
    let mut picked = Picked::default();
    for name in named {
        let (slug, version) = match name.rsplit_once('@') {
            Some((slug, version)) => (slug, Some(version)),
            None => (name.as_str(), None),
        };
        let row = resolve::one(
            "found skill",
            slug,
            listed.iter().copied(),
            slug_of,
            |row| vec![row.key().to_owned(), slug_of(row)],
        )
        .map_err(|error| match error.code {
            ErrorCode::NotFound => error.hint("Run `skillbinder scan` to list them."),
            _ => error,
        })?;
        match (version, row.skill.versions.as_slice()) {
            (None, [_]) => {}
            (None, _) => return Err(several(row)),
            (Some(version), versions) => {
                let chosen = resolve::one(
                    "version",
                    version,
                    versions.iter().copied(),
                    |candidate| short(&candidate.id),
                    |candidate| prefixes(&candidate.id, version),
                )
                .map_err(|error| CommandError {
                    hint: Some(format!("{} has {}.", slug_of(row), listed_versions(row))),
                    ..error
                })?;
                if row.item.is_none_or(|item| item.candidate_id != chosen.id) {
                    picked.versions.push(chosen.id.clone());
                }
            }
        }
        if !picked.keys.iter().any(|key| key == row.key()) {
            picked.keys.push(row.key().to_owned());
        }
    }
    Ok(picked)
}

fn slug_of(row: &Row<'_>) -> String {
    row.shown()
        .map_or_else(|| row.key().to_owned(), |version| version.slug.clone())
}

/// What a version answers to: the start of its digest `typed` gives, of at least four characters.
fn prefixes(id: &CandidateId, typed: &str) -> Vec<String> {
    let typed = typed.trim().to_lowercase();
    if typed.len() >= 4 && id.0.starts_with(&typed) {
        vec![typed]
    } else {
        vec![short(id)]
    }
}

fn listed_versions(row: &Row<'_>) -> String {
    let versions: Vec<String> = row
        .skill
        .versions
        .iter()
        .map(|version| short(&version.id))
        .collect();
    format!("versions {}", versions.join(", "))
}

/// A skill with several versions, named without one.
fn several(row: &Row<'_>) -> CommandError {
    let slug = slug_of(row);
    CommandError::new(
        ErrorCode::Usage,
        format!(
            "{slug} has {} {}.",
            row.skill.versions.len(),
            listed_versions(row)
        ),
    )
    .hint(format!(
        "Name one as {slug}@<version>; `skillbinder scan` shows where each was found."
    ))
}

/// What Import applies: the plan's versions of the picked skills, whether one is invalid, and the
/// copies they replace, as `<slug> in <place>`.
#[derive(Debug, Eq, PartialEq)]
pub(super) struct Chosen {
    pub(super) ids: Vec<CandidateId>,
    pub(super) invalid: bool,
    pub(super) replaces: Vec<String>,
}

/// The picked skills as `found`'s plan imports them. One none of whose versions can be imported,
/// as a blocked one, refuses the import.
pub(super) fn chosen(
    found: &FoundImport,
    snapshot: &AppSnapshot,
    picked: &Picked,
) -> Result<Chosen, CommandError> {
    let groups = groups(found, &snapshot.workspaces);
    let keys: BTreeSet<String> = picked.keys.iter().cloned().collect();
    let taken: Vec<&Row<'_>> = rows(&groups)
        .filter(|row| keys.contains(row.key()))
        .collect();
    if let Some(row) = taken.iter().find(|row| !row.tickable()) {
        let why = match row.shown().map(|version| version.validation) {
            Some(ValidationStatus::Blocked) => "it is blocked",
            _ => "its folder has no files",
        };
        return Err(CommandError::new(
            ErrorCode::Failed,
            format!("{} can't be imported: {why}.", slug_of(row)),
        ));
    }
    let selection = selection(&groups, &keys);
    if selection.over() {
        return Err(CommandError::new(
            ErrorCode::Usage,
            format!("One import takes at most {MAX_IMPORT} skills."),
        ));
    }
    let replaces = taken
        .iter()
        .filter_map(|row| row.item.map(|item| (slug_of(row), item)))
        .flat_map(|(slug, item)| {
            item.replaces
                .iter()
                .map(move |directory| format!("{slug} in {}", place(directory, snapshot)))
        })
        .collect();
    Ok(Chosen {
        ids: selection.ids,
        invalid: selection.invalid,
        replaces,
    })
}

/// Goes ahead with importing `chosen` as `replace` allows, when it replaces copies: in a terminal
/// once the user agrees, with `--replace` at once, and otherwise not at all (spec rule 7).
pub(super) async fn agree(
    chosen: &Chosen,
    replace: Permission,
    peer: &Peer,
) -> Result<(), CommandError> {
    if chosen.replaces.is_empty() {
        return Ok(());
    }
    let list: String = chosen
        .replaces
        .iter()
        .map(|copy| format!("\n  {copy}"))
        .collect();
    let (skills, copies) = (
        count(chosen.ids.len(), "skill", "skills"),
        count(chosen.replaces.len(), "copy", "copies"),
    );
    match replace {
        Permission::Yes => Ok(()),
        Permission::Ask => {
            let prompt = format!(
                "Importing replaces your copies:{list}\nImport {skills} and replace {copies}?"
            );
            if peer.ask(prompt).await {
                Ok(())
            } else {
                Err(CommandError::new(
                    ErrorCode::NeedsAnswer,
                    "Nothing changed.",
                ))
            }
        }
        Permission::No => Err(CommandError::new(
            ErrorCode::NeedsAnswer,
            format!("Importing {skills} replaces {copies}:{list}"),
        )
        .hint("Run again with --replace to import and replace them, or import in SkillBinder.")),
    }
}

fn count(count: usize, one: &str, many: &str) -> String {
    match count {
        1 => format!("1 {one}"),
        count => format!("{count} {many}"),
    }
}

/// Imports what `named` names, or with `all` what the dialog ticks, under the progress strip as
/// the dialog's Import does. Naming a skill with invalid metadata acknowledges it.
pub(super) async fn import(
    named: Vec<String>,
    all: bool,
    replace: Permission,
    peer: &Peer,
    client: &Client,
    cx: &mut AsyncApp,
) -> Result<Changed, CommandError> {
    let mut found = prepare(client, Vec::new()).await?;
    let picked = pick(&found, &client.snapshot().workspaces, &named, all)?;
    if !picked.versions.is_empty() {
        found = prepare(client, picked.versions.clone()).await?;
    }
    let chosen = chosen(&found, &client.snapshot(), &picked)?;
    if chosen.ids.is_empty() {
        return Ok(Changed::said("Found no skills to import."));
    }
    agree(&chosen, replace, peer).await?;
    free(client, cx)?;
    let (report, reported) = async_channel::bounded(1);
    let (worker, plan, skills) = (client.clone(), found.plan.id, chosen.ids.len());
    let work = async move {
        let outcome = worker
            .apply_import(plan, chosen.ids, chosen.invalid)
            .await?;
        let landing = Landing {
            imported: outcome.imported.len(),
        };
        let _ = report.try_send(outcome);
        Ok(landing)
    };
    blocking(client, Activity::Importing { skills }, None, work, cx).await?;
    let installed: Vec<String> = reported
        .recv()
        .await
        .map(|outcome| outcome.imported.iter().map(ToString::to_string).collect())
        .unwrap_or_default();
    Ok(Changed {
        message: format!("Imported {}.", count(installed.len(), "skill", "skills")),
        installed,
        ..Changed::default()
    })
}

#[cfg(test)]
mod tests;
