//! Normalizes `library/skills` to one directory per skill and rebuilds `library/.skillbinder.json`.
//!
//! One-shot repair tool. The layout is `skills/<slug>/`, or `skills/<slug>-<id prefix>/` when two
//! skills share a slug, and the metadata is a cache of payload digests and totals, so both can be
//! rebuilt from the payloads on disk.
//!
//!     cargo run --example reindex -- <library-path>

use serde_json::Value;
use skillbinder_core::library::{
    ValidationLevel, ValidationLimits, ValidationStatus, inspect_payload,
};
use skillbinder_platform::{
    library_repository::payload_directories,
    payload_filesystem::FilesystemPayloadSource,
    portable_metadata::{PortableMetadata, PortableSkill},
};
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

fn main() {
    let library = PathBuf::from(
        env::args()
            .nth(1)
            .expect("usage: cargo run --example reindex -- <library-path>"),
    );
    let path = library.join(".skillbinder.json");
    let before = fs::read_to_string(&path).expect("existing library metadata");
    let existing: Value = serde_json::from_str(&before).expect("existing library metadata is JSON");
    let skills = library.join("skills");

    let payloads = collect_payloads(&skills, &existing);
    let directories = payload_directories(
        payloads
            .iter()
            .map(|payload| (payload.id.as_str(), payload.slug.as_str())),
    );
    let mut placed = Vec::new();
    for payload in &payloads {
        let target = skills.join(&directories[&payload.id]);
        if payload.path != target {
            assert!(
                !target.exists(),
                "{} and {} both want {}",
                payload.path.display(),
                target.display(),
                directories[&payload.id]
            );
            fs::rename(&payload.path, &target).expect("move payload");
        }
        placed.push((payload.id.clone(), payload.slug.clone(), target));
    }
    prune_empty_skill_directories(&skills);

    let mut metadata = PortableMetadata::new(
        existing
            .get("libraryId")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        existing
            .get("createdAt")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
    );
    for (id, slug, path) in &placed {
        let model = inspect_payload(path, &FilesystemPayloadSource, &ValidationLimits::default())
            .expect("payload inspection");
        assert_ne!(
            model.validation.status,
            ValidationStatus::Blocked,
            "{id}/{slug} could not be walked"
        );
        for message in model
            .validation
            .messages
            .iter()
            .filter(|message| message.level != ValidationLevel::Warning)
        {
            eprintln!("{id}/{slug}: {:?} {}", message.code, message.message);
        }
        metadata.skills.insert(
            id.clone(),
            PortableSkill::from_manifest(id.clone(), slug.clone(), &model.manifest),
        );
    }

    metadata.write(&path).expect("write library metadata");
    let after = fs::read_to_string(&path).expect("written library metadata");
    let reloaded = PortableMetadata::load(&path).expect("written library metadata reloads");
    assert_eq!(reloaded.skills.len(), metadata.skills.len());

    println!("payloads {}", placed.len());
    println!("skills {}", metadata.skills.len());
    println!("before {} bytes", before.len());
    println!("after {} bytes", after.len());
}

struct Payload {
    id: String,
    slug: String,
    path: PathBuf,
}

/// A payload is a directory holding `SKILL.md`. Nested layouts name the id in the parent directory,
/// flat ones take the id from the metadata record whose directory the name matches.
fn collect_payloads(skills: &Path, existing: &Value) -> Vec<Payload> {
    let records: BTreeMap<&str, &str> = existing
        .get("skills")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(id, record)| {
            record
                .get("slug")
                .and_then(Value::as_str)
                .map(|slug| (id.as_str(), slug))
        })
        .collect();
    let mut ids: BTreeMap<String, String> =
        payload_directories(records.iter().map(|(id, slug)| (*id, *slug)))
            .into_iter()
            .map(|(id, directory)| (directory, id))
            .collect();

    let mut payloads = Vec::new();
    for entry in fs::read_dir(skills).expect("library skills directory") {
        let entry = entry.expect("skills entry");
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.path().join("SKILL.md").is_file() {
            let id = match ids.remove(&name) {
                Some(id) => id,
                None => panic!("skills/{name} has no metadata record naming its id"),
            };
            let slug = records
                .get(id.as_str())
                .map(|slug| (*slug).to_owned())
                .expect("payload record");
            payloads.push(Payload {
                id,
                slug,
                path: entry.path(),
            });
            continue;
        }
        for inner in fs::read_dir(entry.path()).expect("skill directory") {
            let inner = inner.expect("skill entry").path();
            if inner.join("SKILL.md").is_file() {
                payloads.push(Payload {
                    id: name.clone(),
                    slug: inner.file_name().unwrap().to_string_lossy().into_owned(),
                    path: inner,
                });
            }
        }
    }
    payloads.sort_by(|left, right| left.id.cmp(&right.id));
    payloads
}

fn prune_empty_skill_directories(skills: &Path) {
    for entry in fs::read_dir(skills).expect("library skills directory") {
        let path = entry.expect("skills entry").path();
        if path.is_dir() && !path.join("SKILL.md").is_file() {
            let _ = fs::remove_dir(&path);
        }
    }
}
