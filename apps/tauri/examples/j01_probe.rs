use skillbinder_core::{
    bootstrap::BootstrapEnvironment,
    discovery::{Registry, ResolvedRoot, ScanLimits, ScanOutcome, scan_global_roots},
    import::{
        ImportSelection, ImportService, ImportSnapshot, LibraryRepository, ObservationStore,
        SystemClock, UuidSource,
    },
    library::{ValidationLimits, ValidationStatus},
};
use skillbinder_db::StateStore;
use skillbinder_platform::{
    library_repository::FilesystemLibraryRepository, local_environment::LocalEnvironment,
    paths::AppPaths, payload_filesystem::FilesystemPayloadSource,
};
use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    io::Write,
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
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let executable = home.join(".claude/skills/review/assets/notes.md");
        let mut permissions = fs::metadata(&executable).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(executable, permissions).unwrap();
    }
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
    write_skill(
        &home.join(".claude/linked"),
        "linked",
        "---\nname: linked\ndescription: Linked directory.\n---\n",
    );
    symlink(
        home.join(".claude/linked"),
        home.join(".claude/skills/linked"),
    )
    .unwrap();
    write_skill(
        &home.join(".claude/skills/oversized"),
        "oversized",
        "---\nname: oversized\ndescription: Too large.\n---\n",
    );
    let oversized = home.join(".claude/skills/oversized/SKILL.md");
    fs::OpenOptions::new()
        .append(true)
        .open(&oversized)
        .unwrap()
        .write_all(&vec![b'x'; 1_048_577])
        .unwrap();
    write_skill(
        &home.join(".claude/skills/hardlink"),
        "hardlink",
        "---\nname: hardlink\ndescription: Hardlink.\n---\n",
    );
    fs::hard_link(
        outside.join("secret.txt"),
        home.join(".claude/skills/hardlink/outside.txt"),
    )
    .unwrap();
    write_skill(
        &home.join(".gemini/antigravity-cli/skills/review"),
        "review",
        "---\nname: review\ndescription: Review code for correctness.\n---\nBody\n",
    );
    let duplicate_assets = home.join(".gemini/antigravity-cli/skills/review/assets/notes.md");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&duplicate_assets).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&duplicate_assets, permissions).unwrap();
    }
    for name in ["alpha", "beta", "gamma"] {
        write_skill(
            &home.join(".gemini/antigravity/skills").join(name),
            name,
            &format!("---\nname: {name}\ndescription: Probe fixture {name}.\n---\n"),
        );
    }
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
            || location.state != skillbinder_core::discovery::LocationState::Missing
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
    let linked = find("linked");
    let oversized = find("oversized");
    let hardlink = find("hardlink");
    let linked_ok = outcome
        .candidates
        .iter()
        .filter(|candidate| candidate.slug == "linked")
        .count()
        == 1
        && linked.display_path.ends_with(".claude/linked");
    let oversized_ok = oversized.validation.status == ValidationStatus::Blocked
        && oversized.validation.messages.iter().any(|message| {
            message.code == skillbinder_core::library::ValidationCode::FileLimitExceeded
        });
    let hardlink_ok = hardlink.validation.status == ValidationStatus::Blocked
        && hardlink.validation.messages.iter().any(|message| {
            message.code == skillbinder_core::library::ValidationCode::UnsupportedEntryType
        });
    println!("linked skill discovered and materialized: {linked_ok}");
    println!("oversized SKILL.md blocked: {oversized_ok}");
    println!("hardlink blocked: {hardlink_ok}");
    if !linked_ok {
        problems.push("linked skill was not materialized".into());
    }
    if !oversized_ok {
        problems.push("oversized SKILL.md was not blocked".into());
    }
    if !hardlink_ok {
        problems.push("hardlink was not blocked".into());
    }
    let legacy = outcome
        .candidates
        .iter()
        .find(|candidate| candidate.slug == "legacy");
    println!(
        "outside-home root refused: {}",
        legacy.is_none()
            && outcome.locations.iter().any(|location| {
                location.display_path.contains(".astrbot")
                    && location.state == skillbinder_core::discovery::LocationState::Unreadable
            })
    );
    if legacy.is_some() {
        problems.push("outside-home root symlink was scanned".into());
    }

    let imports = ImportService {
        source,
        plans: store.clone(),
        library: library.clone(),
        observations: store.clone(),
        clock: Arc::new(SystemClock),
        ids: Arc::new(UuidSource),
        limits: ValidationLimits::default(),
    };

    let claude_review = candidate_at(&outcome, ".claude/skills/review");
    let duplicate_review = candidate_at(&outcome, ".gemini/antigravity-cli/skills/review");
    let dedup_selection: Vec<ImportSelection> = [claude_review, duplicate_review]
        .into_iter()
        .map(ImportSelection::from)
        .collect();
    let dedup_snapshot = snapshot_for(library.as_ref(), &dedup_selection);
    let dedup_plan = imports
        .prepare(dedup_selection, false, dedup_snapshot)
        .unwrap();
    let dedup_new = matches!(
        dedup_plan.items[0].decision,
        skillbinder_core::import::ImportDecision::NewSkill { .. }
    );
    let dedup_attach = matches!(
        &dedup_plan.items[1].decision,
        skillbinder_core::import::ImportDecision::AttachObservation { skill_id }
            if *skill_id == dedup_plan.items[0].skill_id
    );
    imports.apply(&dedup_plan.id).unwrap();
    let dedup_observations = store.list(&dedup_plan.items[0].skill_id).unwrap().len();
    let dedup_records = library.catalog().unwrap().len();
    println!(
        "identical candidates in one batch make one skill with two observations: {}",
        dedup_new && dedup_attach && dedup_observations == 2 && dedup_records == 1
    );
    if !(dedup_new && dedup_attach && dedup_observations == 2 && dedup_records == 1) {
        problems.push("batch dedup did not collapse identical candidates".into());
    }

    let copied_review = paths
        .library()
        .join("skills")
        .join(&dedup_plan.items[0].skill_id)
        .join(&dedup_plan.items[0].selection.slug)
        .join("assets/notes.md");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let source_mode =
            fs::metadata(review_hashed_source(&home, "review").join("assets/notes.md"))
                .unwrap()
                .permissions()
                .mode();
        let copied_mode = fs::metadata(&copied_review).unwrap().permissions().mode();
        let executable_preserved = source_mode & 0o111 != 0 && copied_mode & 0o111 != 0;
        println!("executable bit preserved into the library: {executable_preserved}");
        if !executable_preserved {
            problems.push("executable bit was lost while staging".into());
        }
    }

    let revision = library.current_revision().unwrap();
    let linked = candidate_at(&outcome, ".claude/linked");
    let selection: Vec<ImportSelection> = [review, shared, outline, linked]
        .into_iter()
        .map(ImportSelection::from)
        .collect();
    let snapshot = snapshot_for(library.as_ref(), &selection);

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
            library_revision: revision,
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

    let linked_library_copy = paths
        .library()
        .join("skills")
        .join(&plan.items[3].skill_id)
        .join(&plan.items[3].selection.slug)
        .join("SKILL.md");
    let linked_imported = linked_library_copy.is_file();
    println!("linked skill imported once from its original: {linked_imported}");
    if !linked_imported {
        problems.push("linked skill did not import".into());
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

    let alpha = candidate_at(&outcome, "/alpha");
    let beta = candidate_at(&outcome, "/beta");
    let failure_selection: Vec<ImportSelection> = [alpha, beta]
        .into_iter()
        .map(ImportSelection::from)
        .collect();
    let failure_snapshot = snapshot_for(library.as_ref(), &failure_selection);
    let failure_plan = imports
        .prepare(failure_selection, false, failure_snapshot)
        .unwrap();
    let occupied = paths
        .library()
        .join("skills")
        .join(&failure_plan.items[1].skill_id)
        .join(&failure_plan.items[1].selection.slug);
    fs::create_dir_all(&occupied).unwrap();
    fs::write(occupied.join("unrelated.txt"), "keep me\n").unwrap();
    let failure_result = imports.apply(&failure_plan.id);
    let unrelated_kept = occupied.join("unrelated.txt").is_file();
    let first_rolled_back = !paths
        .library()
        .join("skills")
        .join(&failure_plan.items[0].skill_id)
        .exists();
    let failure_records = library.catalog().unwrap();
    let no_partial_records = failure_records.iter().all(|record| {
        record.skill_id != failure_plan.items[0].skill_id
            && record.skill_id != failure_plan.items[1].skill_id
    });
    let journal_clean = library.unresolved_import_journal().unwrap().is_none();
    println!(
        "occupied destination refused, batch rolled back, unrelated content kept: {}",
        failure_result.is_err()
            && unrelated_kept
            && first_rolled_back
            && no_partial_records
            && journal_clean
    );
    println!(
        "  refused={} unrelated_kept={} first_rolled_back={} no_partial_records={} journal_clean={}",
        failure_result.is_err(),
        unrelated_kept,
        first_rolled_back,
        no_partial_records,
        journal_clean
    );
    if !(failure_result.is_err()
        && unrelated_kept
        && first_rolled_back
        && no_partial_records
        && journal_clean)
    {
        problems
            .push("batch failure left partial library state or destroyed unrelated content".into());
    }
    if let Err(error) = &failure_result {
        println!("  batch failure error: {error}");
    }
    fs::remove_dir_all(&occupied).unwrap();

    let gamma = candidate_at(&outcome, "/gamma");
    let change_selection: Vec<ImportSelection> = vec![gamma.into()];
    let change_snapshot = snapshot_for(library.as_ref(), &change_selection);
    let change_plan = imports
        .prepare(change_selection, false, change_snapshot)
        .unwrap();
    fs::write(
        home.join(".gemini/antigravity/skills/gamma/assets/notes.md"),
        "changed after prepare\n",
    )
    .unwrap();
    let change_result = imports.apply(&change_plan.id);
    let change_records = library.catalog().unwrap();
    let source_change_rejected = change_result.is_err()
        && change_records
            .iter()
            .all(|record| record.skill_id != change_plan.items[0].skill_id);
    println!("source change between prepare and apply is rejected: {source_change_rejected}");
    if !source_change_rejected {
        problems.push("a changed source still reached the library".into());
    }

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

fn candidate_at<'a>(
    outcome: &'a ScanOutcome,
    suffix: &str,
) -> &'a skillbinder_core::discovery::ScanCandidate {
    outcome
        .candidates
        .iter()
        .find(|candidate| candidate.display_path.ends_with(suffix))
        .unwrap_or_else(|| panic!("candidate ending in {suffix} missing"))
}

fn snapshot_for(library: &dyn LibraryRepository, selections: &[ImportSelection]) -> ImportSnapshot {
    ImportSnapshot {
        candidate_ids: selections
            .iter()
            .map(|item| item.candidate_id.clone())
            .collect(),
        library_revision: library.current_revision().unwrap(),
        allow_invalid_skills: false,
    }
}

fn review_hashed_source(home: &Path, skill: &str) -> PathBuf {
    home.join(".claude/skills").join(skill)
}
