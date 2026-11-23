//! Explore from a terminal: `search` over the catalog, your repositories, and skills.sh, and
//! `install` into the library, inactive (ADR 0015). The catalog is fetched again first when
//! Explore would fetch it on opening, and installs refuse what Explore's Install refuses.

use super::plan::Plan;
use super::resolve;
use super::run::on_runtime;
use crate::native_view::{
    Local, all_time_label, collides, creator_of, in_library, install_error, installs_label,
    result_collides, result_in_library,
};
use skillbinder_client::Client;
use skillbinder_proto::command_line::{
    Changed, CommandError, ErrorCode, Failure, Output, SearchOutput, SearchRow, SkillsShAnswer,
};
use skillbinder_proto::{
    AppError, AppSnapshot, Catalog, CatalogOrigin, CatalogSkill, ErrorCategory, InstallState,
    Resolution, SkillsShResult, SkillsShState, install_key, normalized_url, owner_repo, result_key,
};

/// `search`: the catalog's and your repositories' matches, then skills.sh's.
pub(super) async fn search(client: &Client, query: &str) -> Result<Output, CommandError> {
    fresh_catalog(client).await;
    let asked = query.to_owned();
    let answered = on_runtime(client, move |client| async move {
        client.ask_skills_sh(&asked).await
    })
    .await?;
    Ok(Output::Search(found(&client.snapshot(), query, &answered)))
}

/// What a search for `query` finds in `snapshot`, with skills.sh's `answered` results not already
/// listed, as Explore shows them.
pub(super) fn found(snapshot: &AppSnapshot, query: &str, answered: &SkillsShState) -> SearchOutput {
    let search = query.trim().to_lowercase();
    let empty = Catalog::default();
    let local = local(snapshot, &empty);
    let listed = local.results(&search, None);
    let shown: Vec<(String, &str)> = listed
        .iter()
        .map(|skill| (normalized_url(&skill.source.url), skill.key.as_str()))
        .collect();
    let mut results: Vec<SearchRow> = listed
        .iter()
        .map(|skill| skill_row(snapshot, local, skill))
        .collect();
    let skills_sh = match answered {
        SkillsShState::Results(found) => {
            results.extend(
                found
                    .iter()
                    .filter(|result| {
                        !shown.contains(&(normalized_url(&result.url), result.key.as_str()))
                    })
                    .map(|result| result_row(snapshot, result)),
            );
            SkillsShAnswer::Answered
        }
        SkillsShState::Idle => SkillsShAnswer::NotAsked,
        SkillsShState::NeedsConnection => SkillsShAnswer::Offline,
        SkillsShState::Searching | SkillsShState::Unavailable => SkillsShAnswer::Unavailable,
    };
    SearchOutput { results, skills_sh }
}

fn local<'a>(snapshot: &'a AppSnapshot, empty: &'a Catalog) -> Local<'a> {
    Local {
        catalog: snapshot.explore.catalog.as_deref().unwrap_or(empty),
        repositories: &snapshot.explore.repositories,
    }
}

fn skill_row(snapshot: &AppSnapshot, local: Local<'_>, skill: &CatalogSkill) -> SearchRow {
    let source = match &skill.origin {
        CatalogOrigin::Curated { .. } => creator_of(local.catalog, skill).map_or_else(
            || owner_repo(&skill.source.url),
            |creator| creator.name.clone(),
        ),
        CatalogOrigin::Repository { .. } => owner_repo(&skill.source.url),
        CatalogOrigin::SkillsSh { .. } => "skills.sh".to_owned(),
    };
    SearchRow {
        reference: reference(skill),
        name: skill.name.clone(),
        description: skill.description.clone(),
        source,
        installs: installs_label(skill),
        in_library: in_library(snapshot, skill),
        collides: collides(snapshot, skill),
    }
}

fn result_row(snapshot: &AppSnapshot, result: &SkillsShResult) -> SearchRow {
    SearchRow {
        reference: format!("{}@{}", owner_repo(&result.url), result.skill_id),
        name: result.name.clone(),
        description: String::new(),
        source: "skills.sh".to_owned(),
        installs: Some(all_time_label(result.installs)),
        in_library: result_in_library(snapshot, result),
        collides: result_collides(snapshot, result),
    }
}

/// `<owner>/<repo>/<slug>`, as `install` takes it.
fn reference(skill: &CatalogSkill) -> String {
    format!("{}/{}", owner_repo(&skill.source.url), skill.slug)
}

/// Fetches the catalog again when Explore's opening would; offline, the cached one stays.
async fn fresh_catalog(client: &Client) {
    let _ = on_runtime(client, |client| async move { client.fresh_catalog().await }).await;
}

/// Installing needs Git, as Explore's Install does.
pub(super) fn needs_git(snapshot: &AppSnapshot, what: &str) -> Result<(), CommandError> {
    match snapshot.bootstrap.git_version {
        Some(_) => Ok(()),
        None => Err(
            CommandError::new(ErrorCode::GitMissing, format!("{what} needs Git."))
                .hint("Install Git, then try again."),
        ),
    }
}

/// `install`: checked for Git now, its references resolved once the catalog is fresh.
pub(super) fn plan_install(
    snapshot: &AppSnapshot,
    skills: &[String],
) -> Result<Plan, CommandError> {
    needs_git(snapshot, "Installing")?;
    Ok(Plan::Install(skills.to_vec()))
}

/// What `install` installs: a catalog's or a repository's skill, or a skills.sh result whose
/// folder is found as it installs.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Target {
    Skill(CatalogSkill),
    SkillsSh(SkillsShResult),
}

/// The skill `reference` names: skills.sh's `<owner>/<repo>@<skill>`, or among the catalog's and
/// your repositories' skills, `<owner>/<repo>/<slug>` or a slug only one of them has.
pub(super) fn target(snapshot: &AppSnapshot, reference: &str) -> Result<Target, CommandError> {
    if let Some((repository, skill)) = reference.split_once('@') {
        return skills_sh(reference, repository, skill).map(Target::SkillsSh);
    }
    let empty = Catalog::default();
    let listed = local(snapshot, &empty).results("", None);
    resolve::one(
        "skill",
        reference,
        listed.iter().copied(),
        self::reference,
        |skill| vec![skill.slug.clone(), self::reference(skill)],
    )
    .cloned()
    .map(Target::Skill)
    .map_err(|error| match error.code {
        ErrorCode::NotFound if error.hint.is_none() => error.hint(
            "Find it with `skillbinder search <query>`, or name a skills.sh skill as \
             <owner>/<repo>@<skill>.",
        ),
        _ => error,
    })
}

/// skills.sh's `<owner>/<repo>@<skill>`, its repository on github.com, as skills.sh names it.
fn skills_sh(
    reference: &str,
    repository: &str,
    skill: &str,
) -> Result<SkillsShResult, CommandError> {
    let usable = |part: &str| {
        !matches!(part, "" | "." | "..")
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
    };
    let skill = skill.trim();
    match repository.split_once('/') {
        Some((owner, repository)) if usable(owner) && usable(repository) && !skill.is_empty() => {
            Ok(SkillsShResult {
                skill_id: skill.to_owned(),
                // Only Explore's marks read it; slugs are ASCII, which folding lowercases.
                key: skill.to_lowercase(),
                name: skill.to_owned(),
                url: format!("https://github.com/{owner}/{repository}.git"),
                installs: 0,
            })
        }
        _ => Err(CommandError::new(
            ErrorCode::Usage,
            format!("\"{reference}\" is not a skills.sh skill."),
        )
        .hint("Write it as <owner>/<repo>@<skill>, as `skillbinder search` prints it.")),
    }
}

/// Installs each of `skills` in turn, as Explore's Install does, stopping at the first that
/// fails with what landed before it.
pub(super) async fn install(skills: Vec<String>, client: &Client) -> Result<Changed, Failure> {
    fresh_catalog(client).await;
    let snapshot = client.snapshot();
    let targets = skills
        .iter()
        .map(|reference| target(&snapshot, reference))
        .collect::<Result<Vec<_>, _>>()?;
    let mut changed = Changed::default();
    let mut held = Vec::new();
    for target in targets {
        let target = match target {
            Target::SkillsSh(result) => Target::SkillsSh(listed(client, result).await),
            target @ Target::Skill(_) => target,
        };
        let installed = match refusal(&client.snapshot(), &target) {
            Ok(Some(slug)) => {
                held.push(slug);
                continue;
            }
            Ok(None) => install_one(client, target).await,
            Err(error) => Err(error),
        };
        match installed {
            Ok(slug) => changed.installed.push(slug),
            Err(error) => {
                changed.message = installed_message(&changed.installed, &held);
                return Err(Failure {
                    error,
                    result: (!changed.installed.is_empty())
                        .then(|| Box::new(Output::Changed(changed))),
                });
            }
        }
    }
    changed.message = installed_message(&changed.installed, &held);
    Ok(changed)
}

/// skills.sh's own result for `result`, from the window's search or by asking skills.sh, so the
/// resolution Explore keeps for the session carries its name and installs; `result` itself when
/// skills.sh doesn't list it.
async fn listed(client: &Client, result: SkillsShResult) -> SkillsShResult {
    let key = result_key(&result);
    let shown = match &client.snapshot().explore.skills_sh.state {
        SkillsShState::Results(shown) => {
            shown.iter().find(|found| result_key(found) == key).cloned()
        }
        _ => None,
    };
    if let Some(shown) = shown {
        return shown;
    }
    let query = result.skill_id.clone();
    let asked = on_runtime(client, move |client| async move {
        client.ask_skills_sh(&query).await
    });
    match asked.await {
        Ok(SkillsShState::Results(found)) => found
            .into_iter()
            .find(|found| result_key(found) == key)
            .unwrap_or(result),
        _ => result,
    }
}

/// Whether Explore would install `target`: its slug when the library holds it from this source
/// already, or a conflict when the library holds its slug from another source or from none.
pub(super) fn refusal(
    snapshot: &AppSnapshot,
    target: &Target,
) -> Result<Option<String>, CommandError> {
    let (slug, held, collides) = match target {
        Target::Skill(skill) => (
            &skill.slug,
            in_library(snapshot, skill),
            collides(snapshot, skill),
        ),
        Target::SkillsSh(result) => (
            &result.skill_id,
            result_in_library(snapshot, result),
            result_collides(snapshot, result),
        ),
    };
    if collides {
        return Err(CommandError::new(
            ErrorCode::Conflict,
            format!("Your library has another skill named {slug}."),
        ));
    }
    Ok(held.then(|| slug.clone()))
}

/// Installs `target`, answering with its slug, or why it failed as Explore's error line says.
async fn install_one(client: &Client, target: Target) -> Result<String, CommandError> {
    let installing = target.clone();
    let installed = on_runtime(client, move |client| async move {
        match installing {
            Target::Skill(skill) => client.install_skill(skill).await,
            Target::SkillsSh(result) => client.install_skills_sh(result).await,
        }
    })
    .await;
    let snapshot = client.snapshot();
    let (skill, named) = match &target {
        Target::Skill(skill) => (Some(skill), &skill.slug),
        Target::SkillsSh(result) => {
            let resolved = match snapshot.explore.resolutions.get(&result_key(result)) {
                Some(Resolution::Resolved(skill)) => Some(skill.as_ref()),
                _ => None,
            };
            (resolved, &result.skill_id)
        }
    };
    let slug = skill.map_or(named, |skill| &skill.slug).clone();
    installed
        .map(|()| slug.clone())
        .map_err(|error| install_failure(&snapshot, skill, &target, &slug, error))
}

/// Why an install failed: the library came to hold the slug, or Explore's error line for it.
fn install_failure(
    snapshot: &AppSnapshot,
    skill: Option<&CatalogSkill>,
    target: &Target,
    slug: &str,
    error: AppError,
) -> CommandError {
    if error.category == ErrorCategory::Stale {
        return CommandError::new(ErrorCode::Conflict, error.message);
    }
    let (key, url) = match (skill, target) {
        (Some(skill), _) => (install_key(&skill.source), &skill.source.url),
        (None, Target::SkillsSh(result)) => (result_key(result), &result.url),
        (None, Target::Skill(skill)) => (install_key(&skill.source), &skill.source.url),
    };
    match snapshot.explore.installs.get(&key) {
        Some(InstallState::Failed(failure)) => {
            CommandError::new(ErrorCode::Failed, install_error(slug, url, failure))
        }
        _ => error.into(),
    }
}

/// `Installed pdf into the library.`, naming those it held already.
fn installed_message(installed: &[String], held: &[String]) -> String {
    let mut lines = Vec::new();
    if !installed.is_empty() {
        lines.push(format!(
            "Installed {} into the library.",
            installed.join(", ")
        ));
    }
    match held {
        [] => {}
        [slug] => lines.push(format!("{slug} is already in the library.")),
        held => lines.push(format!("{} are already in the library.", held.join(", "))),
    }
    lines.join("\n")
}

#[cfg(test)]
pub(super) mod tests;
