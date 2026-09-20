use skillbinder_core::{
    bootstrap::BootstrapEnvironment,
    discovery::{Registry, ResolvedRoot, ScanLimits, scan_global_roots},
    import::{
        ImportSelection, ImportService, ImportSnapshot, LibraryRepository, ObservationStore,
        SystemClock, UuidSource,
    },
    library::{ValidationLimits, ValidationStatus},
};
use skillbinder_db::StateStore;
use skillbinder_platform::{
    bootstrap::LocalEnvironment, library_repository::FilesystemLibraryRepository, paths::AppPaths,
    payload_filesystem::FilesystemPayloadSource,
};
use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    sync::Arc,
};

fn write_skill(root: &Path, name: &str, frontmatter: &str) {
    fs::create_dir_all(root.join("assets")).unwrap();
    fs::write(root.join("SKILL.md"), frontmatter).unwrap();
    fs::write(root.join("assets/notes.md"), "notes\n").unwrap();
    fs::create_dir_all(root.join("empty")).unwrap();
    let _ = name;
}

fn tree_hash(root: &Path) -> u64 {
    let mut entries = Vec::new();
    collect(root, root, &mut entries);
    entries.sort();
    let mut hasher = DefaultHasher::new();
    for (path, bytes) in entries {
        path.hash(&mut hasher);
        bytes.hash(&mut hasher);
    }
    hasher.finish()
}

fn collect(root: &Path, current: &Path, out: &mut Vec<(String, u64)>) {
    for entry in fs::read_dir(current).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).unwrap();
        if metadata.file_type().is_symlink() {
            out.push((path.strip_prefix(root).unwrap().display().to_string(), 0));
        } else if metadata.is_dir() {
            collect(root, &path, out);
        } else {
            let mut hasher = DefaultHasher::new();
            fs::read(&path).unwrap().hash(&mut hasher);
            out.push((
                path.strip_prefix(root).unwrap().display().to_string(),
                hasher.finish(),
            ));
        }
    }
}

fn main() {
    let base = std::env::temp_dir().join(format!("j01-probe-{}", std::process::id()));
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    let home = base.join("home");
    let outside = base.join("outside");
    let paths = AppPaths::new(base.join("data"), base.join("config"), base.join("cache"));
    paths.create_base_directories().unwrap();

    write_skill(
        &home.join(".claude/skills/review"),
        "review",
        "---\nname: review\ndescription: Review code for correctness.\n---\nBody\n",
    );
    write_skill(
        &home.join(".agents/skills/shared"),
        "shared",
        "---\nname: shared\ndescription: Shared by several agents.\n---\n",
    );
    write_skill(
        &home.join(".codex/skills/outline"),
        "outline",
        "---\nname: outline\ndescription: Outline a document.\n---\n",
    );
    write_skill(
        &home.join(".claude/skills/broken"),
        "broken",
        "---\nname: broken\n---\n",
    );
    fs::create_dir_all(home.join(".gemini/antigravity/skills/escape")).unwrap();
    fs::write(
        home.join(".gemini/antigravity/skills/escape/SKILL.md"),
        "---\nname: escape\ndescription: Escaping link.\n---\n",
    )
    .unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("secret.txt"), "outside\n").unwrap();
    symlink(
        &outside,
        home.join(".gemini/antigravity/skills/escape/leak"),
    )
    .unwrap();

    fs::create_dir_all(outside.join("data/skills/legacy")).unwrap();
    fs::write(
        outside.join("data/skills/legacy/SKILL.md"),
        "---\nname: legacy\ndescription: Outside home.\n---\n",
    )
    .unwrap();
    symlink(&outside, home.join(".astrbot")).unwrap();

    let environment = LocalEnvironment::new(paths.clone());
    let completed = environment.complete_local().unwrap();
    println!("library created: {}", completed.library_id);
    println!("library revision: {}", completed.current_revision);

    let registry = Registry::load().unwrap();
    let mut roots = Vec::new();
    for agent in &registry.agents {
        for template in &agent.global_roots {
            if let Some(path) = template.resolve(&home, |_| None).unwrap() {
                roots.push(ResolvedRoot {
                    path,
                    agent_id: agent.id.clone(),
                    agent_label: agent.display_name.clone(),
                });
            }
        }
    }
    println!(
        "registry: {} agents, {} resolved global roots",
        registry.agent_count,
        roots.len()
    );

    let source = Arc::new(FilesystemPayloadSource);
    let library = Arc::new(FilesystemLibraryRepository::new(paths.clone()));
    let store = Arc::new(StateStore::new(paths.database()));

    let outcome = scan_global_roots(
        "probe",
        &roots,
        source.as_ref(),
        library.as_ref(),
        &home,
        ScanLimits::default(),
    );

    println!("\nlocations:");
    for location in &outcome.locations {
        if location.agent_ids.len() > 1
            || location.state != skillbinder_core::discovery::scan::LocationState::Missing
        {
            println!(
                "  {:?} readers={:?} state={:?}{}",
                location.display_path,
                location.agent_ids,
                location.state,
                location
                    .detail
                    .as_ref()
                    .map(|d| format!(" detail={d}"))
                    .unwrap_or_default()
            );
        }
    }

    println!("\ncandidates:");
    for candidate in &outcome.candidates {
        let codes = candidate
            .validation
            .messages
            .iter()
            .map(|m| format!("{:?}", m.code))
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "  {} slug={} status={:?} codes=[{}] readers={:?} files={} bytes={} duplicate={:?} warnings={:?}",
            candidate.candidate_id,
            candidate.slug,
            candidate.validation.status,
            codes,
            candidate.reader_agent_ids,
            candidate.file_count,
            candidate.total_bytes,
            candidate.duplicate,
            candidate.warnings
        );
    }
    println!("limits reached: {}", outcome.limits_reached);

    let find = |slug: &str| {
        outcome
            .candidates
            .iter()
            .find(|candidate| candidate.slug == slug)
            .unwrap_or_else(|| panic!("candidate {slug} missing"))
    };
    let review = find("review");
    let shared = find("shared");
    let outline = find("outline");
    let broken = find("broken");
    let escape = find("escape");

    assert_eq!(broken.validation.status, ValidationStatus::Invalid);
    assert_eq!(escape.validation.status, ValidationStatus::Blocked);
    assert!(shared.reader_agent_ids.len() > 1, "shared root readers");
    let mut problems: Vec<String> = Vec::new();
    let legacy = outcome
        .candidates
        .iter()
        .find(|candidate| candidate.slug == "legacy");
    println!(
        "outside-home root refused: {}",
        legacy.is_none()
            && outcome.locations.iter().any(|location| {
                location.display_path.contains(".astrbot")
                    && location.state
                        == skillbinder_core::discovery::scan::LocationState::Unreadable
            })
    );
    if legacy.is_some() {
        problems.push("outside-home root symlink was scanned".into());
    }

    let imports = ImportService {
        source: source.clone(),
        plans: store.clone(),
        library: library.clone(),
        observations: store.clone(),
        clock: Arc::new(SystemClock),
        ids: Arc::new(UuidSource),
        limits: ValidationLimits::default(),
    };

    let revision = library.current_revision().unwrap();
    let selection: Vec<ImportSelection> = [review, shared, outline]
        .into_iter()
        .map(ImportSelection::from)
        .collect();
    let snapshot = ImportSnapshot {
        candidate_ids: selection
            .iter()
            .map(|item| item.candidate_id.clone())
            .collect(),
        library_revision: revision.clone(),
        allow_invalid_skills: false,
    };

    let invalid_attempt = imports.prepare(
        vec![broken.into()],
        false,
        ImportSnapshot {
            candidate_ids: vec![broken.candidate_id.clone()],
            library_revision: revision.clone(),
            allow_invalid_skills: false,
        },
    );
    println!("\ninvalid candidate refused: {}", invalid_attempt.is_err());
    if invalid_attempt.is_ok() {
        problems.push("invalid candidate was accepted without confirmation".into());
    }

    let blocked_attempt = imports.prepare(
        vec![escape.into()],
        true,
        ImportSnapshot {
            candidate_ids: vec![escape.candidate_id.clone()],
            library_revision: revision.clone(),
            allow_invalid_skills: true,
        },
    );
    println!("blocked candidate refused: {}", blocked_attempt.is_err());
    if blocked_attempt.is_ok() {
        problems.push("blocked candidate was accepted".into());
    }

    let review_before = tree_hash(&review_hashed_source(&home, "review"));
    let plan = imports.prepare(selection, false, snapshot).unwrap();
    println!("\nplan {} items {}", plan.id, plan.items.len());
    for item in &plan.items {
        println!(
            "  {} -> {} decision={:?}",
            item.selection.slug, item.skill_id, item.decision
        );
    }
    let journal_before = library.unresolved_import_journal().unwrap();
    println!("journal before apply: {journal_before:?}");
    let result = imports.apply(&plan.id).unwrap();
    println!("imported {} skills", result.imported.len());
    let journal_after = library.unresolved_import_journal().unwrap();
    println!("journal after apply: {journal_after:?}");
    if journal_after.is_some() {
        problems.push("journal still present after a successful apply".into());
    }

    let review_after = tree_hash(&review_hashed_source(&home, "review"));
    println!("source bytes unchanged: {}", review_before == review_after);
    if review_before != review_after {
        problems.push("source payload changed during import".into());
    }

    println!("\nlibrary tree:");
    for entry in fs::read_dir(paths.library().join("skills")).unwrap() {
        let entry = entry.unwrap();
        for inner in fs::read_dir(entry.path()).unwrap() {
            let inner = inner.unwrap();
            println!(
                "  skills/{}/{}",
                entry.file_name().to_string_lossy(),
                inner.file_name().to_string_lossy()
            );
        }
    }

    let records = library.catalog().unwrap();
    println!("catalog records: {}", records.len());
    for record in &records {
        let observations = store.list(&record.skill_id).unwrap();
        println!(
            "  {} slug={} digest={} files={} observations={:?}",
            record.skill_id,
            record.slug,
            record.digest,
            record
                .manifest
                .entries
                .iter()
                .filter(|e| e.kind == skillbinder_core::library::ManifestKind::File)
                .count(),
            observations
                .iter()
                .map(|o| o.source.display().to_string())
                .collect::<Vec<_>>()
        );
    }

    let second = imports
        .prepare(
            vec![review.into()],
            false,
            ImportSnapshot {
                candidate_ids: vec![review.candidate_id.clone()],
                library_revision: library.current_revision().unwrap(),
                allow_invalid_skills: false,
            },
        )
        .unwrap();
    println!("\nre-import decision: {:?}", second.items[0].decision);
    match second.items[0].decision {
        skillbinder_core::import::ImportDecision::AttachObservation { .. } => {}
        _ => problems.push("identical re-import did not attach an observation".into()),
    }
    let replay = imports.apply(&plan.id).unwrap();
    println!(
        "replayed apply idempotent: {}",
        replay.imported.len() == result.imported.len()
    );
    println!("records after replay: {}", library.catalog().unwrap().len());

    println!(
        "\nPROBE RESULT: {}",
        if problems.is_empty() {
            "PASS".to_string()
        } else {
            format!("FAIL: {}", problems.join("; "))
        }
    );
    if !problems.is_empty() {
        std::process::exit(1);
    }
}

fn review_hashed_source(home: &Path, skill: &str) -> PathBuf {
    home.join(".claude/skills").join(skill)
}
