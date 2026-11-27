//! Skill updates from a terminal: `updates [--check]`, `update [--diff]`, `update --all`, and
//! `reinstall`, as Library's Updates filter, Check now, Update and its review, Update all, and
//! Reinstall run them. All but listing what the last check found need Git (spec rule 5).

use super::plan::Plan;
use super::run::{busy, free, held, on_runtime};
use super::{reads, resolve};
use crate::gate::Gate;
use crate::native_view::{GIT_NEEDED, available, fetch_failed, has_update};
use gpui_kit::AsyncApp;
use skillbinder_client::Client;
use skillbinder_proto::command_line::{
    Changed, CommandError, ErrorCode, Failure, Output, ReviewsOutput, SkillFilter, Unchecked,
    UpdatesOutput,
};
use skillbinder_proto::{
    AppError, AppSnapshot, Drift, LibrarySnapshot, SkillId, UpdateCheck, UpdateOutcome,
};

/// Fails with the app's own words while Git is missing or too old (spec rule 5).
pub(super) fn needs_git(snapshot: &AppSnapshot) -> Result<(), CommandError> {
    match snapshot.bootstrap.git_version {
        Some(_) => Ok(()),
        None => Err(CommandError::new(ErrorCode::GitMissing, GIT_NEEDED)
            .hint("Install Git, then try again.")),
    }
}

/// `updates`: the skills with an update, and those whose source the last check couldn't read.
pub(super) fn updates(
    snapshot: &AppSnapshot,
    library: &LibrarySnapshot,
) -> Result<UpdatesOutput, CommandError> {
    let unchecked = library
        .library
        .skills
        .iter()
        .filter_map(|skill| match &snapshot.updates.get(&skill.id)?.check {
            UpdateCheck::Failed(reason) => Some(Unchecked {
                slug: skill.id.to_string(),
                reason: reason.clone(),
            }),
            UpdateCheck::UpToDate | UpdateCheck::Available => None,
        })
        .collect();
    Ok(UpdatesOutput {
        skills: reads::skills(snapshot, library, SkillFilter::Updates, None)?,
        unchecked,
    })
}

/// `updates --check`: Check now, then what it found.
pub(super) async fn check(client: &Client) -> Result<Output, CommandError> {
    needs_git(&client.snapshot())?;
    client.check_now().await?;
    let snapshot = client.snapshot();
    Ok(Output::Updates(updates(
        &snapshot,
        resolve::library(&snapshot)?,
    )?))
}

/// `update <slug>… --diff`: the changes Update would land in each named skill with an update,
/// as its review dialog shows them, built one after another behind any running change; nothing
/// is written.
pub(super) async fn diff(client: &Client, skills: &[String]) -> Result<Output, CommandError> {
    let snapshot = client.snapshot();
    let (skills, current) = with_update(&snapshot, resolve::library(&snapshot)?, skills)?;
    let mut reviews = Vec::with_capacity(skills.len());
    for id in skills {
        let fetched = id.clone();
        let review = on_runtime(client, move |client| async move {
            client.review_update(fetched).await
        })
        .await
        .map_err(|error| unfetched(&id, error))?;
        reviews.push(review);
    }
    Ok(Output::Reviews(ReviewsOutput {
        reviews,
        current: current.iter().map(ToString::to_string).collect(),
    }))
}

/// Why `id`'s changes can't be shown, as its review dialog says it: the fetch's or the
/// refusal's own words.
fn unfetched(id: &SkillId, error: AppError) -> CommandError {
    CommandError {
        message: fetch_failed(id, &error.message),
        ..error.into()
    }
}

/// `update`: the named skills with an update, as their Update buttons offer; the others have
/// none to take.
pub(super) fn plan_update(
    snapshot: &AppSnapshot,
    library: &LibrarySnapshot,
    skills: &[String],
) -> Result<Plan, CommandError> {
    let (skills, current) = with_update(snapshot, library, skills)?;
    if skills.is_empty() {
        return Ok(Plan::Save {
            saves: Vec::new(),
            done: no_update(&current),
        });
    }
    Ok(Plan::Update { skills, current })
}

/// The named skills with an update, then those without one; both need Git.
fn with_update(
    snapshot: &AppSnapshot,
    library: &LibrarySnapshot,
    skills: &[String],
) -> Result<(Vec<SkillId>, Vec<SkillId>), CommandError> {
    needs_git(snapshot)?;
    Ok(resolve::skills(library, skills)?
        .into_iter()
        .map(|skill| skill.id.clone())
        .partition(|id| has_update(snapshot, id)))
}

/// `update --all`: nothing to do while no skill has an update, as Library then offers no
/// Update all.
pub(super) fn plan_update_all(snapshot: &AppSnapshot) -> Result<Plan, CommandError> {
    needs_git(snapshot)?;
    if available(snapshot).is_empty() {
        return Ok(Plan::Save {
            saves: Vec::new(),
            done: "No skill has an update.".to_owned(),
        });
    }
    Ok(Plan::UpdateAll)
}

/// `reinstall`: a drifted skill whose recorded content can be restored, as its page offers
/// Reinstall.
pub(super) fn plan_reinstall(
    snapshot: &AppSnapshot,
    library: &LibrarySnapshot,
    slug: &str,
) -> Result<Plan, CommandError> {
    needs_git(snapshot)?;
    let skill = resolve::skill(library, slug)?;
    match skill.drift {
        None => Ok(Plan::Save {
            saves: Vec::new(),
            done: format!("{} matches its record.", skill.id),
        }),
        Some(Drift::Restorable) => Ok(Plan::Reinstall(skill.id.clone())),
        Some(Drift::Lost) => Err(CommandError::new(
            ErrorCode::Failed,
            format!("{} can't be reinstalled.", skill.id),
        )
        .hint(
            "Neither the library's history nor the skill's source holds its recorded content any \
             more.",
        )),
    }
}

/// Updates `skills` from their sources, refusing other changes meanwhile as the window does.
pub(super) async fn update(
    skills: Vec<SkillId>,
    current: Vec<SkillId>,
    client: &Client,
    cx: &mut AsyncApp,
) -> Result<Changed, Failure> {
    free(client, cx)?;
    let updating = on_runtime(client, move |client| async move {
        client.update_skills(skills).await
    });
    let outcome = held(cx, updating).await?;
    updated(&outcome, &current, &client.snapshot())
}

/// Update all, refused as the window and the menu refuse it: while a change or a Git operation
/// runs. It counts itself on the snapshot, so the window's header and the menu show it.
pub(super) async fn update_all(client: &Client, cx: &mut AsyncApp) -> Result<Changed, Failure> {
    let snapshot = client.snapshot();
    let refused = cx.update(|cx| {
        let gate = Gate::of(cx);
        gate.refuses_update_all(&snapshot)
            .then(|| busy(&gate, &snapshot))
    });
    if let Some(refused) = refused {
        return Err(refused.into());
    }
    let outcome = client.update_all().await?;
    updated(&outcome, &[], &client.snapshot())
}

/// Restores `skill`'s recorded content, refusing other changes meanwhile.
pub(super) async fn reinstall(
    skill: SkillId,
    client: &Client,
    cx: &mut AsyncApp,
) -> Result<Changed, CommandError> {
    free(client, cx)?;
    let done = format!("Reinstalled {skill}.");
    let reinstalled = on_runtime(client, move |client| async move {
        client.reinstall_skill(skill).await
    });
    held(cx, reinstalled).await?;
    Ok(Changed::said(done))
}

/// What an update did: the skills updated, then those that failed, with why, beside them.
pub(super) fn updated(
    outcome: &UpdateOutcome,
    current: &[SkillId],
    snapshot: &AppSnapshot,
) -> Result<Changed, Failure> {
    let mut message = match outcome.updated.as_slice() {
        [] => "No skill was updated.".to_owned(),
        updated => format!("Updated {}.", slugs(updated)),
    };
    if !current.is_empty() {
        message = format!("{message} {}", no_update(current));
    }
    let changed = Changed {
        message,
        updated: outcome.updated.iter().map(ToString::to_string).collect(),
        ..Changed::default()
    };
    if outcome.failed.is_empty() {
        return Ok(changed);
    }
    let reasons: Vec<String> = outcome
        .failed
        .iter()
        .map(|id| {
            let failure = snapshot
                .updates
                .get(id)
                .and_then(|update| update.failure.as_deref());
            format!(
                "{id}: {}",
                failure.unwrap_or("it is no longer in the library.")
            )
        })
        .collect();
    let failed = match outcome.failed.len() {
        1 => "1 skill couldn't be updated".to_owned(),
        count => format!("{count} skills couldn't be updated"),
    };
    Err(Failure {
        error: CommandError::new(ErrorCode::Failed, failed).hint(reasons.join("\n")),
        result: Some(Box::new(Output::Changed(changed))),
    })
}

/// `pdf has no update.` or `pdf, sql have no update.`
fn no_update(skills: &[SkillId]) -> String {
    match skills {
        [_] => format!("{} has no update.", slugs(skills)),
        _ => format!("{} have no update.", slugs(skills)),
    }
}

fn slugs(skills: &[SkillId]) -> String {
    skills
        .iter()
        .map(SkillId::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests;
