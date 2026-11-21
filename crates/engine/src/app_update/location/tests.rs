#![expect(
    clippy::unwrap_used,
    reason = "Test fixture failures must abort the test."
)]

use super::*;
use std::fs;

const RUNS_FROM: &str = "/Users/me/Downloads/SkillBinder.app";

fn placement(root: &Path) -> AppPlacement {
    AppPlacement {
        bundle: None,
        applications: root.join("Applications"),
        user_applications: root.join("me/Applications"),
    }
}

fn outside(bundle: &str, mounts: Mounts) -> Origin {
    let placement = placement(Path::new("/"));
    match classify(Path::new(bundle), &placement, || mounts) {
        Location::OutsideApplications(original) => {
            assert_eq!(original.bundle, Path::new(bundle));
            original.origin
        }
        other => panic!("{bundle} is outside Applications, not {other:?}"),
    }
}

#[test]
fn the_bundle_holds_the_executable() {
    assert_eq!(
        bundle_of(Path::new(
            "/Applications/SkillBinder.app/Contents/MacOS/skillbinder"
        )),
        Some(PathBuf::from("/Applications/SkillBinder.app"))
    );
    for unbundled in [
        "/Users/me/SkillBinder/target/debug/skillbinder",
        "/Users/me/Odd.app/target/debug/skillbinder",
        "/skillbinder",
    ] {
        assert_eq!(bundle_of(Path::new(unbundled)), None, "{unbundled}");
    }
}

#[test]
fn a_folder_is_outside_applications() {
    for folder in [RUNS_FROM, "/Users/me/Desktop/SkillBinder.app"] {
        assert_eq!(outside(folder, Mounts::default()), Origin::Folder);
    }
    // A volume that is not a disk image, such as an external drive, is only a folder.
    assert_eq!(
        outside("/Volumes/Backup/SkillBinder.app", Mounts::default()),
        Origin::Folder
    );
}

#[test]
fn the_disk_image_is_outside_applications_and_ejects() {
    let mounts = Mounts {
        images: vec![PathBuf::from("/Volumes/SkillBinder")],
        nulls: Vec::new(),
    };
    assert_eq!(
        outside("/Volumes/SkillBinder/SkillBinder.app", mounts),
        Origin::DiskImage(PathBuf::from("/Volumes/SkillBinder"))
    );
}

#[test]
fn a_translocated_bundle_is_outside_applications_and_trashes_its_original() {
    let runs = "/private/var/folders/x/T/AppTranslocation/6E1A/d/SkillBinder.app";
    let mount = PathBuf::from("/private/var/folders/x/T/AppTranslocation/6E1A");
    for source in [RUNS_FROM, "/Users/me/Downloads"] {
        let mounts = Mounts {
            images: Vec::new(),
            nulls: vec![(PathBuf::from(source), mount.clone())],
        };
        assert_eq!(
            outside(runs, mounts),
            Origin::Translocated(Some(PathBuf::from(RUNS_FROM))),
            "mounted from {source}"
        );
    }
    assert_eq!(outside(runs, Mounts::default()), Origin::Translocated(None));
}

#[test]
fn a_bundle_anywhere_in_either_applications_folder_is_installed() {
    let root = tempfile::tempdir().unwrap();
    let placement = placement(root.path());
    let utilities = placement.applications.join("Utilities");
    for folder in [
        &placement.applications,
        &placement.user_applications,
        &utilities,
    ] {
        fs::create_dir_all(folder).unwrap();
        let bundle = folder.join("SkillBinder.app");
        let location = classify(&bundle, &placement, || panic!("mounts are not read"));
        assert_eq!(location, Location::Installed(bundle));
    }
}

#[cfg(unix)]
#[test]
fn a_bundle_in_a_read_only_applications_folder_is_not_writable() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let placement = placement(root.path());
    fs::create_dir_all(&placement.applications).unwrap();
    let read_only = fs::Permissions::from_mode(0o555);
    fs::set_permissions(&placement.applications, read_only).unwrap();
    if persistence::writable(&placement.applications) {
        // Permissions don't bind this user, such as root in a container.
        return;
    }
    let bundle = placement.applications.join("SkillBinder.app");
    let location = classify(&bundle, &placement, Mounts::default);
    fs::set_permissions(&placement.applications, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(location, Location::NotWritable);
}

#[test]
fn mounts_are_read_from_hdiutil_and_mount() {
    let info = "================================================\n\
        image-path      : /Users/me/Downloads/skillbinder-0.2.0.dmg\n\
        /dev/disk8\tGUID_partition_scheme\t\n\
        /dev/disk8s1\tApple_HFS\t/Volumes/SkillBinder\n\
        /dev/disk9s1\t41504653-0000-11AA-AA11-00306543ECAC\t/Volumes/Other Image\n";
    assert_eq!(
        image_volumes(info),
        [
            PathBuf::from("/Volumes/SkillBinder"),
            PathBuf::from("/Volumes/Other Image")
        ]
    );
    let table = "/dev/disk3s1s1 on / (apfs, sealed, local, read-only, journaled)\n\
        /Users/me/Downloads/SkillBinder.app on /private/var/folders/x/T/AppTranslocation/6E1A (nullfs, local, nodev, nosuid, read-only, noowners, quarantine, mounted by me)\n";
    assert_eq!(
        null_mounts(table),
        [(
            PathBuf::from(RUNS_FROM),
            PathBuf::from("/private/var/folders/x/T/AppTranslocation/6E1A")
        )]
    );
}
