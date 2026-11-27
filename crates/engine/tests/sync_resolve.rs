//! A skill both devices changed to different versions (ADR 0039): Sync refuses until it has a
//! choice for each, keeps the version chosen, and History can restore the other. Both devices
//! share one local bare remote.
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
use skillbinder_proto::{
    ChangeKind, ChangedSkill, Choice, Conflict, ConflictSide, Reason, Side, SkillId, SyncAction,
    SyncState,
};
use std::collections::BTreeMap;

/// Studio Mac pushes its `alpha`, and Laptop changes its own without pulling it.
async fn conflicting() -> Pair {
    let pair = Pair::new().await;
    pair.studio.edit("alpha", "Studio's alpha");
    pair.studio
        .engine
        .synchronize(SyncAction::Push)
        .await
        .unwrap();
    pair.laptop.edit("alpha", "Laptop's alpha");
    pair
}

fn choose(slug: &str, side: Side) -> BTreeMap<String, Side> {
    BTreeMap::from([(slug.to_owned(), side)])
}

#[tokio::test]
async fn sync_refuses_until_each_skill_changed_on_both_sides_has_a_choice() {
    let pair = conflicting().await;
    pair.laptop.edit("beta", "Laptop's beta");
    let conflict = pair.laptop.refresh().await;
    assert_eq!(conflict.state, SyncState::NeedsResolve);
    let [
        Conflict {
            slug,
            local,
            remote,
        },
    ] = &conflict.conflicts[..]
    else {
        panic!("one conflict: {:?}", conflict.conflicts);
    };
    assert_eq!(slug, "alpha");
    let changed = |device: Option<&str>, at| ConflictSide {
        kind: ChangeKind::Changed,
        reason: Some(Reason::Outside),
        device: device.map(str::to_owned),
        at,
    };
    assert_eq!(*local, changed(None, None));
    assert_eq!(*remote, changed(Some("Studio Mac"), remote.at));
    assert!(remote.at.is_some());

    let published = pair.published();
    let missing = "Choose which version to keep of each skill changed on both sides.";
    let refused = pair.laptop.engine.synchronize(SyncAction::Sync).await;
    assert_eq!(refused.unwrap_err().message, missing);
    let refused = pair.laptop.engine.resolve_sync(BTreeMap::new()).await;
    assert_eq!(refused.unwrap_err().message, missing);
    let mut stale = choose("alpha", Side::Local);
    stale.insert("beta".into(), Side::Local);
    let refused = pair.laptop.engine.resolve_sync(stale).await;
    assert_eq!(
        refused.unwrap_err().message,
        "The skills changed on both sides changed. Choose again."
    );
    // Nothing was written: the change still waits, and the remote is as it was.
    assert_eq!(pair.published(), published);
    assert!(pair.laptop.sync().await.uncommitted);
    assert_eq!(pair.laptop.body("alpha").unwrap(), "Laptop's alpha");
}

#[tokio::test]
async fn keeping_this_device_s_version_publishes_it_and_the_remote_s_stays_restorable() {
    let pair = conflicting().await;
    let synced = (pair.laptop.engine)
        .resolve_sync(choose("alpha", Side::Local))
        .await
        .unwrap();
    assert_eq!(synced.state, SyncState::Synced);
    assert_eq!(pair.laptop.body("alpha").unwrap(), "Laptop's alpha");
    assert_eq!(pair.published(), pair.laptop.head());

    pair.studio
        .engine
        .synchronize(SyncAction::Pull)
        .await
        .unwrap();
    assert_eq!(pair.studio.body("alpha").unwrap(), "Laptop's alpha");
    for side in [&pair.studio, &pair.laptop] {
        let history = side.history().await;
        assert_eq!(history[0].device.as_deref(), Some("Laptop"));
        assert_eq!(
            history[0].skills,
            [ChangedSkill {
                slug: "alpha".into(),
                kind: ChangeKind::Changed,
                reason: Some(Reason::Outside),
            }]
        );
        assert_eq!(
            history[0].choices,
            [Choice {
                slug: "alpha".into(),
                side: Side::Local,
                device: Some("Laptop".into()),
                unpicked: None,
            }]
        );
        assert_eq!(history[1].device.as_deref(), Some("Studio Mac"));
    }
    // The studio's version is in its own commit, which History restores.
    let studio = pair.studio.history().await[1].commit.clone();
    (pair.studio.engine)
        .restore_skill(SkillId("alpha".into()), studio)
        .await
        .unwrap();
    assert_eq!(pair.studio.body("alpha").unwrap(), "Studio's alpha");
}

#[tokio::test]
async fn taking_the_remote_s_version_keeps_this_device_s_restorable_from_the_merge() {
    let pair = conflicting().await;
    pair.laptop.refresh().await;
    let synced = (pair.laptop.engine)
        .resolve_sync(choose("alpha", Side::Remote))
        .await
        .unwrap();
    assert_eq!(synced.state, SyncState::Synced);
    assert_eq!(pair.laptop.body("alpha").unwrap(), "Studio's alpha");
    let subject = git(&pair.laptop.library(), &["log", "-1", "--format=%s"]);
    assert_eq!(subject.trim(), "Laptop: sync with the remote");

    let merge = pair.laptop.history().await.remove(0);
    assert!(merge.skills.is_empty());
    let [choice] = &merge.choices[..] else {
        panic!("one choice: {:?}", merge.choices);
    };
    assert_eq!(
        (choice.side, choice.device.as_deref()),
        (Side::Remote, Some("Studio Mac"))
    );
    let unpicked = choice.unpicked.clone().unwrap();
    let second = git(&pair.laptop.library(), &["rev-parse", "HEAD^2"]);
    assert_eq!(unpicked, second.trim());
    (pair.laptop.engine)
        .restore_skill(SkillId("alpha".into()), unpicked)
        .await
        .unwrap();
    assert_eq!(pair.laptop.body("alpha").unwrap(), "Laptop's alpha");
}
