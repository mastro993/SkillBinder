//! A change that writes and removes copies counts the (directory, skill) pairs its passes bring in
//! line, while other jobs leave the counts alone. Every agent directory is under an isolated home.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/fixture.rs"]
mod support;

use skillbinder_engine::Engine;
use skillbinder_proto::{
    ActivationProgress, LibrarySnapshot, OrganizationChange, PresetId, SkillId,
};
use std::{fs, path::PathBuf};
use support::{Fixture, scan, write_skill};

/// A library holding `slugs`, none active, with Claude Code's directory and the universal one
/// detected.
struct Agents {
    _fixture: Fixture,
    engine: Engine,
    claude: PathBuf,
    universal: PathBuf,
    /// The counts the import that filled the library reported.
    imported: ActivationProgress,
}

impl Agents {
    async fn with(slugs: &[&str]) -> Self {
        let fixture = Fixture::new();
        let claude = fixture.home.join(".claude/skills");
        let universal = fixture.home.join(".agents/skills");
        for directory in [&claude, &universal] {
            fs::create_dir_all(directory).unwrap();
        }
        let engine = fixture.ready().await;
        for slug in slugs {
            fixture.skill(slug, "Library version");
        }
        let workspace = fixture.register(&engine).await;
        let ids = fixture
            .scan(&engine)
            .await
            .candidates
            .into_iter()
            .map(|candidate| candidate.id)
            .collect();
        let plan = engine.prepare_import(ids, false).await.unwrap();
        engine.apply_import(plan.id).await.unwrap();
        let imported = engine.activation_progress().unwrap().unwrap();
        // Importing made the skills active in the workspace they were found in; these journeys
        // start with none active, so the workspace goes and leaves its copies unmanaged.
        engine.remove_workspace(workspace.id).await.unwrap();
        Self {
            _fixture: fixture,
            engine,
            claude,
            universal,
            imported,
        }
    }

    async fn organize(&self, change: OrganizationChange) -> LibrarySnapshot {
        self.engine.change_organization(change).await.unwrap()
    }

    /// A new preset holding `slugs` for all your agents, not active yet.
    async fn preset(&self, name: &str, slugs: &[&str]) -> PresetId {
        let library = self
            .organize(OrganizationChange::CreatePreset { name: name.into() })
            .await;
        let id = library
            .library
            .presets
            .iter()
            .find(|preset| preset.name == name)
            .unwrap()
            .id
            .clone();
        if !slugs.is_empty() {
            self.organize(OrganizationChange::AddToPreset {
                id: id.clone(),
                skills: slugs.iter().map(|slug| skill(slug)).collect(),
            })
            .await;
        }
        id
    }

    fn progress(&self) -> ActivationProgress {
        self.engine.activation_progress().unwrap().unwrap()
    }
}

fn skill(slug: &str) -> SkillId {
    SkillId(slug.to_owned())
}

fn counts(done: usize, total: usize) -> ActivationProgress {
    ActivationProgress {
        done,
        total,
        step: None,
    }
}

/// How many of the library's copies need the user.
fn needing_attention(library: &LibrarySnapshot) -> usize {
    library
        .copies
        .iter()
        .filter(|copy| copy.state.needs_attention())
        .count()
}

#[tokio::test]
async fn a_switch_counts_each_skill_in_each_directory() {
    let agents = Agents::with(&["alpha", "beta", "gamma"]).await;
    // The import adopted one copy per skill where it was found.
    assert_eq!(agents.imported, counts(3, 3));
    let work = agents.preset("Work", &["alpha", "beta", "gamma"]).await;
    agents
        .organize(OrganizationChange::ActivatePreset { id: work })
        .await;
    assert_eq!(agents.progress(), counts(6, 6));
    assert_eq!(agents.progress().percent(), 100);
    agents.engine.shutdown().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn problems_never_stop_the_pass() {
    use std::os::unix::fs::PermissionsExt;
    let agents = Agents::with(&["alpha", "beta", "gamma"]).await;
    let work = agents.preset("Work", &["alpha", "beta", "gamma"]).await;
    write_skill(&agents.universal, "alpha", "Their version");
    fs::set_permissions(&agents.claude, fs::Permissions::from_mode(0o555)).unwrap();
    if fs::write(agents.claude.join("probe"), "").is_ok() {
        // Permissions do not bind this user, such as root.
        return;
    }
    let library = agents
        .organize(OrganizationChange::ActivatePreset { id: work })
        .await;
    fs::set_permissions(&agents.claude, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(agents.progress(), counts(6, 6));
    // The universal alpha differs, and none of Claude Code's three could be written.
    assert_eq!(needing_attention(&library), 4);
    agents.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn an_edited_copy_the_switch_removes_counts_and_waits() {
    let agents = Agents::with(&["alpha"]).await;
    agents
        .organize(OrganizationChange::AddToPreset {
            id: PresetId("default".to_owned()),
            skills: vec![skill("alpha")],
        })
        .await;
    write_skill(&agents.claude, "alpha", "Edited here");
    let empty = agents.preset("Empty", &[]).await;
    let library = agents
        .organize(OrganizationChange::ActivatePreset { id: empty })
        .await;
    assert_eq!(agents.progress(), counts(2, 2));
    assert_eq!(needing_attention(&library), 1);
    assert!(agents.claude.join("alpha").exists(), "the edits stay");
    assert!(!agents.universal.join("alpha").exists());
    agents.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn deleting_the_active_preset_counts_the_switch_to_default() {
    let agents = Agents::with(&["alpha", "beta"]).await;
    let work = agents.preset("Work", &["alpha", "beta"]).await;
    agents
        .organize(OrganizationChange::ActivatePreset { id: work.clone() })
        .await;
    let preview = agents.engine.preview_delete(work.clone()).await.unwrap();
    agents
        .organize(OrganizationChange::DeletePreset {
            id: work,
            revision: preview.revision,
        })
        .await;
    assert_eq!(agents.progress(), counts(4, 4));
    assert!(!agents.claude.join("beta").exists());
    agents.engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn later_scans_and_retries_leave_the_counts_alone() {
    let agents = Agents::with(&["alpha", "beta"]).await;
    let work = agents.preset("Work", &["alpha", "beta"]).await;
    agents
        .organize(OrganizationChange::ActivatePreset { id: work })
        .await;
    let switched = agents.progress();
    assert_eq!(switched, counts(4, 4));
    // A deleted copy makes the next scan's Pending pass and a retry write it again.
    fs::remove_dir_all(agents.claude.join("alpha")).unwrap();
    scan(&agents.engine).await;
    agents.engine.retry_copies().await.unwrap();
    assert!(agents.claude.join("alpha").exists());
    assert_eq!(agents.progress(), switched);
    agents.engine.shutdown().await.unwrap();
}
