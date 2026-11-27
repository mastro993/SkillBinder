//! Sync merges per skill (ADR 0039) on two devices that share one local bare remote: changes to
//! different skills merge on their own, an identical change is no conflict, and a merge never
//! publishes a tree a fetch would refuse.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/device.rs"]
mod device;
#[path = "support/pair.rs"]
mod pair;
#[path = "support/fixture.rs"]
mod support;

use device::git;
use pair::Pair;
use skillbinder_proto::{ChangeKind, ChangedSkill, Reason, SyncAction, SyncState};
use std::fs;

fn outside(slug: &str) -> ChangedSkill {
    ChangedSkill {
        slug: slug.to_owned(),
        kind: ChangeKind::Changed,
        reason: Some(Reason::Outside),
    }
}

#[tokio::test]
async fn devices_that_changed_different_skills_both_sync() {
    let pair = Pair::new().await;
    pair.studio.edit("alpha", "Studio's alpha");
    pair.studio
        .engine
        .synchronize(SyncAction::Push)
        .await
        .unwrap();
    pair.laptop.edit("beta", "Laptop's beta");

    let both = pair.laptop.refresh().await;
    assert_eq!(both.state, SyncState::NeedsSync);
    assert!(both.conflicts.is_empty());
    let synced = pair
        .laptop
        .engine
        .synchronize(SyncAction::Sync)
        .await
        .unwrap();
    assert_eq!(synced.state, SyncState::Synced);
    assert_eq!(pair.laptop.body("alpha").unwrap(), "Studio's alpha");
    assert_eq!(pair.laptop.body("beta").unwrap(), "Laptop's beta");
    assert_eq!(pair.published(), pair.laptop.head());

    let pulled = pair
        .studio
        .engine
        .synchronize(SyncAction::Sync)
        .await
        .unwrap();
    assert_eq!(pulled.state, SyncState::Synced);
    assert_eq!(pair.studio.body("beta").unwrap(), "Laptop's beta");

    // The merge is one change by the device that made it, with what it contributed and why;
    // each device's published commits follow it on both.
    for side in [&pair.studio, &pair.laptop] {
        let history = side.history().await;
        let devices: Vec<_> = history.iter().map(|c| c.device.as_deref()).collect();
        assert_eq!(devices[..2], [Some("Laptop"), Some("Studio Mac")]);
        assert_eq!(history[0].skills, [outside("beta")]);
        assert!(history[0].choices.is_empty());
        assert_eq!(history[1].skills, [outside("alpha")]);
    }
    let subject = git(&pair.laptop.library(), &["log", "-1", "--format=%s%n%P"]);
    let mut lines = subject.lines();
    assert_eq!(lines.next(), Some("Laptop: update beta"));
    assert_eq!(lines.next().unwrap().split(' ').count(), 2);
}

#[tokio::test]
async fn an_identical_change_on_both_sides_is_no_conflict() {
    let pair = Pair::new().await;
    pair.studio.edit("alpha", "Same");
    pair.studio
        .engine
        .synchronize(SyncAction::Push)
        .await
        .unwrap();
    pair.laptop.edit("alpha", "Same");
    pair.laptop.edit("beta", "Laptop's beta");

    let both = pair.laptop.refresh().await;
    assert_eq!(both.state, SyncState::NeedsSync);
    assert!(both.conflicts.is_empty());
    pair.laptop
        .engine
        .synchronize(SyncAction::Sync)
        .await
        .unwrap();
    // The merge takes the identical skill from the remote, so it credits only what it adds.
    assert_eq!(pair.laptop.history().await[0].skills, [outside("beta")]);
    assert_eq!(pair.laptop.body("alpha").unwrap(), "Same");
}

#[tokio::test]
async fn a_merge_that_a_fetch_would_refuse_is_never_published() {
    let pair = Pair::new().await;
    // Each name is portable alone; together they collide on a case-insensitive disk.
    pair.studio.edit("Gamma", "Studio's");
    pair.studio
        .engine
        .synchronize(SyncAction::Push)
        .await
        .unwrap();
    pair.laptop.edit("gamma", "Laptop's");
    let published = pair.published();

    assert_eq!(pair.laptop.refresh().await.state, SyncState::NeedsSync);
    let refused = pair.laptop.engine.synchronize(SyncAction::Sync).await;
    assert_eq!(
        refused.unwrap_err().message,
        "Skill directory names collide or are not portable."
    );
    assert_eq!(pair.published(), published);
    assert_eq!(pair.laptop.body("gamma").unwrap(), "Laptop's");
    // The change waits on this device, committed but not pushed.
    let waiting = pair.laptop.refresh().await;
    assert_eq!(waiting.state, SyncState::NeedsSync);
    assert_eq!(waiting.local[0].slug, "gamma");
}

#[tokio::test]
async fn adoption_still_takes_precedence_over_a_merge() {
    let pair = Pair::new().await;
    // The laptop adopted the remote: its history is the studio's, with no merge.
    assert_eq!(pair.laptop.head(), pair.published());
    let merges = git(&pair.laptop.library(), &["rev-list", "--merges", "HEAD"]);
    assert_eq!(merges, "");
    assert_eq!(pair.laptop.body("alpha").unwrap(), "First version");
}

#[tokio::test]
async fn a_merge_needs_no_reflog() {
    let pair = Pair::new().await;
    let library = pair.laptop.library();
    git(&library, &["config", "core.logAllRefUpdates", "false"]);
    fs::remove_dir_all(library.join(".git/logs")).unwrap();
    pair.studio.edit("alpha", "Studio's alpha");
    pair.studio
        .engine
        .synchronize(SyncAction::Push)
        .await
        .unwrap();
    pair.laptop.edit("beta", "Laptop's beta");

    pair.laptop.refresh().await;
    let synced = pair
        .laptop
        .engine
        .synchronize(SyncAction::Sync)
        .await
        .unwrap();
    assert_eq!(synced.state, SyncState::Synced);
    assert_eq!(pair.published(), pair.laptop.head());
    assert!(!library.join(".git/logs/HEAD").exists());
}

#[tokio::test]
async fn a_slug_with_glob_characters_takes_only_its_own_skill() {
    let pair = Pair::new().await;
    pair.studio.edit("beta", "Studio's beta");
    pair.studio
        .engine
        .synchronize(SyncAction::Push)
        .await
        .unwrap();
    // As a pathspec, `alph[a]` would also match `alpha` and its record.
    pair.laptop.edit("alph[a]", "Laptop's bracketed skill");

    pair.laptop.refresh().await;
    pair.laptop
        .engine
        .synchronize(SyncAction::Sync)
        .await
        .unwrap();
    let published = git(&pair.remote, &["ls-tree", "-r", "--name-only", "main"]);
    for path in [
        "skills/alpha/SKILL.md",
        "records/alpha.toml",
        "skills/alph[a]/SKILL.md",
    ] {
        assert!(
            published.lines().any(|line| line == path),
            "{path}: {published}"
        );
    }
    assert_eq!(pair.laptop.body("alpha").unwrap(), "First version");
}
