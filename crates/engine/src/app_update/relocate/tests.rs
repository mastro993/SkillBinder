#![expect(
    clippy::unwrap_used,
    reason = "Test fixture failures must abort the test."
)]

use super::*;

/// A fixture SkillBinder bundle at `bundle` recording `version`, or no version, as builds before
/// Move to Applications.
fn bundle_at(bundle: &Path, version: Option<&str>, marker: &str) {
    fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
    fs::write(bundle.join("Contents/MacOS/skillbinder"), marker).unwrap();
    let key = version
        .map(|version| format!("<key>{VERSION_KEY}</key><string>{version}</string>\n"))
        .unwrap_or_default();
    let plist = format!("<plist version=\"1.0\"><dict>\n{key}</dict></plist>\n");
    fs::write(bundle.join("Contents/Info.plist"), plist).unwrap();
}

fn marker(bundle: &Path) -> String {
    fs::read_to_string(bundle.join("Contents/MacOS/skillbinder")).unwrap()
}

/// Copies `from`'s contents into the folder `into`, as `ditto` does.
fn copy(from: &Path, into: &Path) -> io::Result<()> {
    fs::create_dir_all(into)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = into.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

struct Fixture {
    _root: tempfile::TempDir,
    original: PathBuf,
    placement: AppPlacement,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let original = root.path().join("Downloads/SkillBinder.app");
        bundle_at(&original, Some("0.2.0"), "running");
        let placement = AppPlacement {
            bundle: Some(original.clone()),
            executable: None,
            applications: root.path().join("Applications"),
            user_applications: root.path().join("me/Applications"),
        };
        fs::create_dir(&placement.applications).unwrap();
        Self {
            _root: root,
            original,
            placement,
        }
    }

    fn relocate(&self) -> io::Result<PathBuf> {
        relocate(
            &self.original,
            &self.placement,
            "0.2.0".parse().unwrap(),
            copy,
        )
    }

    /// The names in `folder`, so a test sees no staging or set-aside bundle is left.
    fn names(folder: &Path) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

#[test]
fn the_bundle_is_copied_into_applications() {
    let fixture = Fixture::new();
    let moved = fixture.relocate().unwrap();
    assert_eq!(moved, fixture.placement.applications.join(NAME));
    assert_eq!(marker(&moved), "running");
    assert_eq!(version_of(&moved), Some("0.2.0".parse().unwrap()));
    assert_eq!(Fixture::names(&fixture.placement.applications), [NAME]);
    assert!(fixture.original.exists(), "the caller disposes of it");
}

#[cfg(unix)]
#[test]
fn a_read_only_applications_folder_falls_back_to_the_users() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    let system = &fixture.placement.applications;
    fs::set_permissions(system, fs::Permissions::from_mode(0o555)).unwrap();
    if crate::persistence::writable(system) {
        // Permissions don't bind this user, such as root in a container.
        return;
    }
    let moved = fixture.relocate();
    fs::set_permissions(system, fs::Permissions::from_mode(0o755)).unwrap();
    let user = &fixture.placement.user_applications;
    assert_eq!(moved.unwrap(), user.join(NAME));
    assert_eq!(Fixture::names(user), [NAME]);
    assert!(Fixture::names(system).is_empty());
}

#[test]
fn an_older_copy_is_replaced() {
    for older in [Some("0.1.9"), Some("0.2.0-beta.3"), Some("0.2.0"), None] {
        let fixture = Fixture::new();
        let there = fixture.placement.applications.join(NAME);
        bundle_at(&there, older, "older");
        assert_eq!(fixture.relocate().unwrap(), there);
        assert_eq!(marker(&there), "running", "{older:?} is replaced");
        assert_eq!(Fixture::names(&fixture.placement.applications), [NAME]);
    }
}

#[test]
fn a_newer_copy_is_kept_and_opened() {
    for newer in ["0.2.1-beta.1", "1.0.0"] {
        let fixture = Fixture::new();
        let there = fixture.placement.applications.join(NAME);
        bundle_at(&there, Some(newer), "newer");
        assert_eq!(fixture.relocate().unwrap(), there);
        assert_eq!(marker(&there), "newer", "{newer} stays");
        assert_eq!(Fixture::names(&fixture.placement.applications), [NAME]);
    }
}

#[test]
fn a_failed_copy_keeps_the_copy_already_there() {
    let fixture = Fixture::new();
    let there = fixture.placement.applications.join(NAME);
    bundle_at(&there, Some("0.1.0"), "older");
    let failed = relocate(
        &fixture.original,
        &fixture.placement,
        "0.2.0".parse().unwrap(),
        |_, _| Err(io::Error::other("disk full")),
    );
    assert!(failed.is_err());
    assert_eq!(marker(&there), "older");
    assert_eq!(Fixture::names(&fixture.placement.applications), [NAME]);
}

#[cfg(target_os = "macos")]
#[test]
fn ditto_copies_the_bundle_without_its_quarantine() {
    let fixture = Fixture::new();
    let quarantine = |action: &str, bundle: &Path| {
        let mut xattr = Command::new("/usr/bin/xattr");
        match action {
            "-w" => xattr.args(["-w", "com.apple.quarantine", "0081;00000000;Safari;"]),
            _ => xattr.args(["-p", "com.apple.quarantine"]),
        };
        xattr.arg(bundle.join("Contents/Info.plist"));
        run(&mut xattr)
    };
    quarantine("-w", &fixture.original).unwrap();
    let moved = relocate(
        &fixture.original,
        &fixture.placement,
        "0.2.0".parse().unwrap(),
        ditto,
    )
    .unwrap();
    assert_eq!(marker(&moved), "running");
    assert!(quarantine("-p", &fixture.original).is_ok());
    assert!(
        quarantine("-p", &moved).is_err(),
        "the copy is not quarantined"
    );
}
