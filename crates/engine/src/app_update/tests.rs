use super::{
    RELEASE_KEY,
    check::LastCheck,
    manifest::Offer,
    running_version,
    settings::{CHANNEL, LAST_CHECK},
};
use crate::worker::State;
use skillbinder_proto::Channel;

#[test]
fn the_workspace_version_is_an_app_version() {
    assert_eq!(running_version().to_string(), env!("CARGO_PKG_VERSION"));
}

#[test]
fn the_release_key_is_a_minisign_public_key() {
    assert!(minisign_verify::PublicKey::from_base64(RELEASE_KEY).is_ok());
}

#[test]
#[expect(
    clippy::unwrap_used,
    reason = "Test fixture failures must abort the test."
)]
fn a_last_check_of_another_channel_is_not_restored() {
    let temp = tempfile::tempdir().unwrap();
    let state = State::isolated_test_state(temp.path().to_path_buf());
    // A Stable check that ended as the user switched to Beta, saved after the switch.
    state.set_setting(CHANNEL, &Channel::Beta).unwrap();
    let stale = LastCheck {
        channel: Channel::Stable,
        checked: 1_800_000_000,
        offer: Some(Offer {
            version: "9.0.0".parse().unwrap(),
            notes_url: "https://example.invalid/v9.0.0".to_owned(),
        }),
    };
    state.set_setting(LAST_CHECK, &stale).unwrap();
    state.restore_app_updates().unwrap();
    let update = state.app_updates.current().unwrap();
    assert_eq!(update.channel, Channel::Beta);
    assert_eq!(update.last_checked, None);
}
