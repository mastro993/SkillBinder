use super::running_version;

#[test]
fn the_workspace_version_is_an_app_version() {
    assert_eq!(running_version().to_string(), env!("CARGO_PKG_VERSION"));
}
