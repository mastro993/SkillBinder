//! History through the engine: each published change with its device and skills, read from the
//! commit the snapshot's `head` names. Every remote is a local bare repository.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/device.rs"]
mod device;
#[path = "support/fixture.rs"]
mod support;

use device::Device;
use skillbinder_proto::{ChangeKind, ChangedSkill, Reason, SyncAction};
use support::write_skill;

#[tokio::test]
async fn history_lists_each_device_s_changes_from_the_head() {
    let device = Device::published(&["alpha"]).await;
    device.peer_push_with(
        &[
            "Laptop: add beta",
            "SkillBinder-Device: Laptop\nSkillBinder-Skill: beta installed",
        ],
        |skills| {
            write_skill(skills, "beta", "Added there");
        },
    );
    device.engine.synchronize(SyncAction::Pull).await.unwrap();
    let sync = device.engine.snapshot().await.unwrap().sync;

    let history = device.engine.sync_history(None, 50).await.unwrap();
    let devices: Vec<_> = history.changes.iter().map(|c| c.device.clone()).collect();
    assert_eq!(devices, [Some("Laptop".to_owned()), Some(sync.device)]);
    let skills: Vec<_> = history.changes.iter().map(|c| c.skills.clone()).collect();
    assert_eq!(
        skills,
        [
            vec![ChangedSkill {
                slug: "beta".into(),
                kind: ChangeKind::Added,
                reason: Some(Reason::Installed),
            }],
            vec![ChangedSkill {
                slug: "alpha".into(),
                kind: ChangeKind::Added,
                reason: Some(Reason::Imported {
                    from: Some("workspace".into()),
                }),
            }],
        ]
    );
    assert_eq!(sync.head, Some(history.changes[0].commit.clone()));
    assert_eq!(history.next, None);
}
