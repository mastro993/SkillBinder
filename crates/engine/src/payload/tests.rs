#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
use super::*;
use skillbinder_proto::ErrorCategory;

fn source() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("example");
    fs::create_dir(&path).unwrap();
    fs::write(
        path.join("SKILL.md"),
        "---\nname: example\ndescription: A useful skill\n---\nBody\n",
    )
    .unwrap();
    (root, path)
}
#[test]
fn materialization_preserves_source_and_manifest() {
    let (root, path) = source();
    fs::create_dir(path.join("empty")).unwrap();
    fs::write(path.join("data"), b"data").unwrap();
    let inspected = inspect(&path).unwrap();
    assert_eq!(inspected.status, ValidationStatus::Valid);
    let destination = root.path().join("output");
    materialize(&inspected, &destination).unwrap();
    assert_eq!(inspect(&destination).unwrap().manifest, inspected.manifest);
    assert_eq!(inspect(&path).unwrap(), inspected);
    assert!(destination.join("empty").is_dir());
}
#[test]
fn changed_source_refuses_stage_without_destination() {
    let (root, path) = source();
    let inspected = inspect(&path).unwrap();
    fs::write(path.join("new"), b"new").unwrap();
    let destination = root.path().join("output");
    assert_eq!(
        materialize(&inspected, &destination).unwrap_err().category,
        ErrorCategory::Stale
    );
    assert!(!destination.exists());
}
#[test]
fn explicit_directory_changes_digest() {
    let (_root, retained) = source();
    let before = inspect(&retained).unwrap().manifest;
    fs::create_dir(retained.join("empty")).unwrap();
    assert_ne!(inspect(&retained).unwrap().manifest.digest, before.digest);
}
#[test]
fn unsafe_names_and_plugin_manifests_block() {
    for name in ["CON.txt", "trailing.", "colon:name", "bad\\name"] {
        let (_root, path) = source();
        if fs::write(path.join(name), b"x").is_ok() {
            assert_eq!(
                inspect(&path).unwrap().status,
                ValidationStatus::Blocked,
                "{name}"
            );
        }
    }
    let (_root, path) = source();
    fs::create_dir(path.join(".codex-plugin")).unwrap();
    fs::write(path.join(".codex-plugin/plugin.json"), b"{}").unwrap();
    assert_eq!(inspect(&path).unwrap().status, ValidationStatus::Blocked);
}
#[test]
fn git_metadata_is_excluded_with_warning() {
    let (_root, path) = source();
    fs::create_dir(path.join(".git")).unwrap();
    fs::write(path.join(".git/config"), b"private").unwrap();
    let inspected = inspect(&path).unwrap();
    assert_eq!(inspected.status, ValidationStatus::Warning);
    assert_eq!(inspected.manifest.file_count(), 1);
}
#[test]
fn malformed_yaml_features_and_duplicate_keys_are_invalid() {
    for yaml in [
        "name: example\nname: example\ndescription: x",
        "{name: example, 'name': example, description: x}",
        "name: &name example\ndescription: *name",
        "name: !!str example\ndescription: x",
        "name: example\ndescription: x\na: !custom value",
    ] {
        let (_root, path) = source();
        fs::write(path.join("SKILL.md"), format!("---\n{yaml}\n---\n")).unwrap();
        assert_eq!(
            inspect(&path).unwrap().status,
            ValidationStatus::Invalid,
            "{yaml}"
        );
    }
}
#[test]
fn yaml_depth_and_node_limits_are_checked_before_loading() {
    for yaml in [
        format!(
            "name: example\ndescription: x\na: {}0{}",
            "[".repeat(40),
            "]".repeat(40)
        ),
        format!("name: example\ndescription: x\na: [{}]", "0,".repeat(10001)),
    ] {
        let (_root, path) = source();
        fs::write(path.join("SKILL.md"), format!("---\n{yaml}\n---")).unwrap();
        assert_eq!(inspect(&path).unwrap().status, ValidationStatus::Invalid);
    }
}
#[test]
fn oversized_skill_blocks() {
    let (_root, path) = source();
    fs::write(path.join("SKILL.md"), vec![b'x'; 1024 * 1024 + 1]).unwrap();
    assert_eq!(inspect(&path).unwrap().status, ValidationStatus::Blocked);
}
#[test]
fn unicode_casefold_collisions_block() {
    let (_root, path) = source();
    fs::write(path.join("STRASSE"), b"a").unwrap();
    fs::write(path.join("straße"), b"b").unwrap();
    assert_eq!(
        traversal::collision_key("STRASSE"),
        traversal::collision_key("straße")
    );
    // Case-insensitive filesystems may already coalesce these two paths.
    if fs::read_dir(&path).unwrap().count() == 3 {
        assert_eq!(inspect(&path).unwrap().status, ValidationStatus::Blocked);
    }
}
#[cfg(unix)]
mod unix {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};
    #[test]
    fn internal_links_materialize_as_regular_entries() {
        let (root, path) = source();
        fs::create_dir(path.join("assets")).unwrap();
        fs::write(path.join("assets/data"), b"data").unwrap();
        symlink("assets", path.join("linked")).unwrap();
        symlink("assets/data", path.join("data-link")).unwrap();
        let inspected = inspect(&path).unwrap();
        assert_eq!(inspected.status, ValidationStatus::Valid);
        let destination = root.path().join("output");
        materialize(&inspected, &destination).unwrap();
        assert!(
            !fs::symlink_metadata(destination.join("linked"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read(destination.join("linked/data")).unwrap(), b"data");
        assert_eq!(inspect(&path).unwrap(), inspected);
    }
    #[test]
    fn external_dangling_cyclic_and_traversal_links_block() {
        for target in ["../outside", "missing", "link", ".", "assets/../../outside"] {
            let (root, path) = source();
            fs::write(root.path().join("outside"), b"secret").unwrap();
            fs::create_dir(path.join("assets")).unwrap();
            symlink(target, path.join("link")).unwrap();
            let inspected = inspect(&path).unwrap();
            assert_eq!(inspected.status, ValidationStatus::Blocked, "{target}");
            assert!(materialize(&inspected, &root.path().join("output")).is_err());
        }
    }
    #[test]
    fn intermediate_target_links_cannot_escape() {
        let (root, path) = source();
        fs::create_dir(root.path().join("outside")).unwrap();
        fs::write(root.path().join("outside/secret"), b"secret").unwrap();
        symlink("../outside", path.join("redirect")).unwrap();
        symlink("redirect/secret", path.join("link")).unwrap();
        assert_eq!(inspect(&path).unwrap().status, ValidationStatus::Blocked);
    }
    #[test]
    fn executable_bit_is_hashed_and_preserved() {
        let (root, path) = source();
        fs::write(path.join("run"), b"echo hi").unwrap();
        let before = inspect(&path).unwrap().manifest;
        fs::set_permissions(path.join("run"), fs::Permissions::from_mode(0o755)).unwrap();
        let inspected = inspect(&path).unwrap();
        assert_ne!(inspected.manifest.digest, before.digest);
        let destination = root.path().join("output");
        materialize(&inspected, &destination).unwrap();
        assert_ne!(
            fs::metadata(destination.join("run"))
                .unwrap()
                .permissions()
                .mode()
                & 0o111,
            0
        );
    }
    #[test]
    fn identical_replacement_directory_is_stale() {
        let (root, path) = source();
        let inspected = inspect(&path).unwrap();
        fs::rename(&path, root.path().join("old")).unwrap();
        fs::create_dir(&path).unwrap();
        fs::copy(root.path().join("old/SKILL.md"), path.join("SKILL.md")).unwrap();
        assert_eq!(
            materialize(&inspected, &root.path().join("output"))
                .unwrap_err()
                .category,
            ErrorCategory::Stale
        );
    }
}

#[test]
fn schema_one_digest_matches_fixed_binary_encoding() {
    let manifest = Manifest::from_entries(vec![
        ManifestEntry {
            kind: ManifestKind::File,
            path: "x".into(),
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
            bytes: 3,
            executable: false,
        },
        ManifestEntry {
            kind: ManifestKind::Directory,
            path: "empty".into(),
            sha256: String::new(),
            bytes: 0,
            executable: false,
        },
    ])
    .unwrap();
    assert_eq!(
        manifest.digest,
        "sha256:51228e17befc062c79e53a59e3c17d25e868fc5989ace779db4621735b1ba90d"
    );
}
#[test]
fn depth_entry_and_total_byte_limits_block() {
    let (_root, path) = source();
    let mut nested = path.clone();
    for _ in 0..33 {
        nested.push("d");
        fs::create_dir(&nested).unwrap();
    }
    assert_eq!(inspect(&path).unwrap().status, ValidationStatus::Blocked);

    let (_root, path) = source();
    for n in 0..5000 {
        fs::write(path.join(format!("file{n}")), []).unwrap();
    }
    assert_eq!(inspect(&path).unwrap().status, ValidationStatus::Blocked);

    let (_root, path) = source();
    for n in 0..3 {
        fs::write(path.join(format!("large{n}")), vec![0; 9 * 1024 * 1024]).unwrap();
    }
    assert_eq!(inspect(&path).unwrap().status, ValidationStatus::Blocked);
}
#[test]
fn frontmatter_required_fields_and_warning_thresholds() {
    for (yaml, status) in [
        ("description: x".to_owned(), ValidationStatus::Invalid),
        (
            "name: other\ndescription: x".to_owned(),
            ValidationStatus::Invalid,
        ),
        ("name: example".to_owned(), ValidationStatus::Invalid),
        (
            format!("name: example\ndescription: {}", "x".repeat(1025)),
            ValidationStatus::Warning,
        ),
        (
            format!(
                "name: example\ndescription: x\nother: {}",
                "x".repeat(65_536)
            ),
            ValidationStatus::Invalid,
        ),
    ] {
        let (_root, path) = source();
        fs::write(path.join("SKILL.md"), format!("---\n{yaml}\n---\n")).unwrap();
        assert_eq!(inspect(&path).unwrap().status, status);
    }
}
#[test]
fn staging_inside_source_or_existing_destination_is_refused() {
    let (root, path) = source();
    let inspected = inspect(&path).unwrap();
    assert!(materialize(&inspected, &path.join("copy")).is_err());
    fs::create_dir(root.path().join("existing")).unwrap();
    assert!(materialize(&inspected, &root.path().join("existing")).is_err());
    assert_eq!(inspect(&path).unwrap(), inspected);
}
#[cfg(unix)]
#[test]
fn link_hop_limit_blocks_chain_over_sixteen() {
    use std::os::unix::fs::symlink;
    let (_root, path) = source();
    for n in 0..17 {
        symlink(
            if n == 16 {
                "SKILL.md".to_owned()
            } else {
                format!("link{}", n + 1)
            },
            path.join(format!("link{n}")),
        )
        .unwrap();
    }
    assert_eq!(inspect(&path).unwrap().status, ValidationStatus::Blocked);
}
#[cfg(unix)]
#[test]
fn unsupported_fifo_is_blocked_without_opening_it() {
    let (_root, path) = source();
    assert!(
        std::process::Command::new("mkfifo")
            .arg(path.join("fifo"))
            .status()
            .unwrap()
            .success()
    );
    assert_eq!(inspect(&path).unwrap().status, ValidationStatus::Blocked);
}

#[test]
fn malformed_manifest_hash_is_rejected_without_panicking() {
    for digest in ["", "abc", "nothex", &"z".repeat(64), &"é".repeat(32)] {
        assert!(
            Manifest::from_entries(vec![ManifestEntry {
                kind: ManifestKind::File,
                path: "SKILL.md".into(),
                sha256: digest.into(),
                bytes: 1,
                executable: false,
            }])
            .is_err()
        );
    }
}
