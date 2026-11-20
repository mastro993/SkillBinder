use super::{RELEASE_KEY, running_version};

#[test]
fn the_workspace_version_is_an_app_version() {
    assert_eq!(running_version().to_string(), env!("CARGO_PKG_VERSION"));
}

#[test]
fn the_release_key_is_a_minisign_public_key() {
    assert!(minisign_verify::PublicKey::from_base64(RELEASE_KEY).is_ok());
}
