fn main() {
    #[cfg(unix)]
    probe::main();
    #[cfg(not(unix))]
    {
        println!("PROBE RESULT: SKIPPED (requires a Unix host)");
    }
}

#[cfg(unix)]
mod probe {
    use skillbinder_core::{
        discovery::{
            ExclusionReason, LibraryCatalog, LocationState, Registry, ScanInput, ScanLimits,
            ScanOutcome, ScanPolicy, ScanProgress, ScanRoot, project_scan_inputs, scan_roots,
        },
        source::PayloadSource,
    };
    use skillbinder_db::StateStore;
    use skillbinder_platform::payload_filesystem::FilesystemPayloadSource;
    use std::{
        error::Error,
        fs,
        os::unix::fs::{PermissionsExt, symlink},
        path::{Path, PathBuf},
        sync::atomic::{AtomicBool, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    const SKILL: &str = "---\nname: probe\ndescription: probe payload\n---\nbody\n";
    const FULL_CANDIDATES: usize = 6;

    pub fn main() {
        match run() {
            Ok(()) => println!("\nPROBE RESULT: PASS"),
            Err(error) => {
                eprintln!("\nPROBE RESULT: FAIL: {error}");
                std::process::exit(1);
            }
        }
    }

    fn run() -> Result<(), Box<dyn Error>> {
        let base = std::env::temp_dir().join(format!(
            "skillbinder-j02-probe-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&base);
        let grant = base.join("grant");
        build_tree(&grant)?;
        let _cleanup = Cleanup {
            base: base.clone(),
            locked: grant.join(".claude/skills/unreadable"),
        };

        let source = FilesystemPayloadSource;
        let canonical_grant = source.canonicalize_root(&grant)?;
        let store = StateStore::new(base.join("state.sqlite"));
        store.insert_scan_root(&ScanRoot {
            id: "probe-root".into(),
            canonical_path: canonical_grant.clone(),
            display_path: grant.display().to_string(),
            label: "Probe root".into(),
            enabled: true,
            created_at: 1,
        })?;
        store.insert_scan_root(&ScanRoot {
            id: "gone-root".into(),
            canonical_path: base.join("gone"),
            display_path: base.join("gone").display().to_string(),
            label: "Gone root".into(),
            enabled: true,
            created_at: 2,
        })?;
        let registered = store.list_scan_roots()?;
        if registered.len() != 2 || registered[0].canonical_path != canonical_grant {
            return Err("the root store did not round-trip the registered roots".into());
        }
        println!("registered root: {}", registered[0].label);
        println!(
            "resolved path:   {}",
            registered[0].canonical_path.display()
        );

        let registry = Registry::load()?;
        let roots = project_scan_inputs(&registered[0], &registry, &source);
        let gone = project_scan_inputs(&registered[1], &registry, &source);
        println!("known skill directories: {}", roots.len());

        let mut inputs = roots.clone();
        inputs.extend(gone);
        let full = scan(&source, &inputs, None)?;
        print_outcome("full scan", &full);
        assert_full_scan(&full)?;

        let limited_inputs = roots
            .iter()
            .map(|input| ScanInput {
                policy: ScanPolicy::project_with_limits(ScanLimits {
                    category_depth: 12,
                    max_entries: 4,
                    max_link_hops: 16,
                }),
                ..input.clone()
            })
            .collect::<Vec<_>>();
        let limited = scan(&source, &limited_inputs, None)?;
        print_outcome("budget scan", &limited);
        if !limited.limits_reached {
            return Err("a tiny entry budget did not set limits_reached".into());
        }
        if !limited
            .locations
            .iter()
            .any(|location| location.limit_reached)
        {
            return Err("a tiny entry budget did not set limit_reached on the location".into());
        }

        let cancel = AtomicBool::new(false);
        let cancelled = scan(&source, &roots, Some(&cancel))?;
        print_outcome("cancelled scan", &cancelled);
        if !cancelled.cancelled {
            return Err("the cancelled scan did not report cancelled".into());
        }
        if cancelled.candidates.is_empty() {
            return Err("the cancelled scan kept none of the candidates it had found".into());
        }
        if cancelled.candidates.len() >= full.candidates.len() {
            return Err("the cancelled scan did not stop early".into());
        }
        println!(
            "note: cancelled runs keep their candidates in the run but write no import session \
             (covered by the discovery unit test)"
        );
        Ok(())
    }

    fn scan(
        source: &FilesystemPayloadSource,
        inputs: &[ScanInput],
        cancel: Option<&AtomicBool>,
    ) -> Result<ScanOutcome, Box<dyn Error>> {
        let flag = AtomicBool::new(false);
        let requested = cancel.is_some();
        let cancel = cancel.unwrap_or(&flag);
        let mut sink = |progress: ScanProgress| {
            if requested && progress.candidates_found >= 1 {
                cancel.store(true, Ordering::Relaxed);
            }
        };
        Ok(scan_roots(
            "probe",
            inputs,
            source,
            &NoCatalog,
            Path::new("/probe-home"),
            cancel,
            &mut sink,
        ))
    }

    struct NoCatalog;
    impl LibraryCatalog for NoCatalog {
        fn matching_payload(&self, _digest: &str) -> Option<(String, String)> {
            None
        }
        fn slug_owner(&self, _slug: &str) -> Option<(String, String)> {
            None
        }
    }

    fn assert_full_scan(outcome: &ScanOutcome) -> Result<(), Box<dyn Error>> {
        let slugs = outcome
            .candidates
            .iter()
            .map(|candidate| candidate.slug.as_str())
            .collect::<Vec<_>>();
        for expected in ["alpha", "p2", "p3", "p4", "p5", "payload"] {
            if !slugs.contains(&expected) {
                return Err(
                    format!("the full scan missed the {expected} project: {slugs:?}").into(),
                );
            }
        }
        if slugs.len() != FULL_CANDIDATES {
            return Err(format!("the full scan found unexpected candidates: {slugs:?}").into());
        }
        if slugs.contains(&"decoy") {
            return Err("the vendored decoy skill was scanned".into());
        }
        if slugs.contains(&"loose") {
            return Err("a skill outside the known skill directories was scanned".into());
        }
        let decoy = outcome
            .candidates
            .iter()
            .find(|candidate| candidate.display_path.contains("vendor"));
        if decoy.is_some() {
            return Err("a candidate was found below the excluded vendor directory".into());
        }
        let payload = outcome
            .candidates
            .iter()
            .find(|candidate| candidate.slug == "payload")
            .ok_or("the payload skill was not found")?;
        if !payload.linked {
            return Err("the payload reached through a symlink is not marked linked".into());
        }
        let linked = outcome
            .candidates
            .iter()
            .filter(|candidate| candidate.slug == "payload")
            .count();
        if linked != 1 {
            return Err(format!("the payload was reported {linked} times").into());
        }
        let vendor = outcome
            .exclusions
            .iter()
            .find(|exclusion| exclusion.name == "vendor")
            .ok_or("the vendor exclusion was not reported")?;
        if vendor.reason != ExclusionReason::DependencyVendor || vendor.matches < 1 {
            return Err(format!("the vendor exclusion is malformed: {vendor:?}").into());
        }
        if outcome
            .exclusions
            .iter()
            .all(|exclusion| exclusion.name != ".git")
        {
            return Err("the worktree .git marker was not excluded".into());
        }
        if !outcome
            .warnings
            .iter()
            .any(|warning| warning.message.contains("unreadable"))
        {
            return Err("the unreadable directory produced no access warning".into());
        }
        if !outcome
            .locations
            .iter()
            .any(|location| location.state == LocationState::Missing)
        {
            return Err(
                "the unavailable registered root produced no partial location entry".into(),
            );
        }
        if outcome
            .locations
            .iter()
            .filter(|location| location.state == LocationState::Scanned)
            .count()
            != 1
        {
            return Err("the full scan did not report exactly one scanned root".into());
        }
        Ok(())
    }

    fn print_outcome(label: &str, outcome: &ScanOutcome) {
        println!("\n{label}:");
        println!(
            "  locations: {}  candidates: {}  warnings: {}  limits: {}  cancelled: {}",
            outcome.locations.len(),
            outcome.candidates.len(),
            outcome.warnings.len(),
            outcome.limits_reached,
            outcome.cancelled
        );
        for location in &outcome.locations {
            println!(
                "  location {:?} limit_reached={} path={}",
                location.state, location.limit_reached, location.display_path
            );
        }
        for candidate in &outcome.candidates {
            println!(
                "  candidate {} linked={} path={}",
                candidate.slug, candidate.linked, candidate.display_path
            );
        }
        for exclusion in &outcome.exclusions {
            println!(
                "  exclusion {} {:?} matches={} sample={}",
                exclusion.name,
                exclusion.reason,
                exclusion.matches,
                exclusion.sample_path.display()
            );
        }
        for warning in &outcome.warnings {
            println!("  warning {:?}: {}", warning.path, warning.message);
        }
    }

    fn build_tree(grant: &Path) -> Result<(), Box<dyn Error>> {
        let skills = grant.join(".claude/skills");

        let alpha = skills.join("alpha");
        fs::create_dir_all(&alpha)?;
        fs::write(alpha.join("SKILL.md"), SKILL)?;

        let beta = skills.join("beta");
        fs::create_dir_all(&beta)?;
        fs::write(
            beta.join("README.md"),
            "worktree checkout without a skill\n",
        )?;
        fs::write(beta.join(".git"), "gitdir: /work/worktrees/beta\n")?;

        for name in ["p2", "p3", "p4", "p5"] {
            let sibling = skills.join(name);
            fs::create_dir_all(&sibling)?;
            fs::write(sibling.join("SKILL.md"), SKILL)?;
        }

        let decoy = skills.join("vendor/decoy");
        fs::create_dir_all(&decoy)?;
        fs::write(decoy.join("SKILL.md"), SKILL)?;

        let payload = skills.join("payload");
        fs::create_dir_all(&payload)?;
        fs::write(payload.join("SKILL.md"), SKILL)?;
        symlink("payload", skills.join("alias"))?;

        let unreadable = skills.join("unreadable");
        fs::create_dir_all(&unreadable)?;
        fs::write(unreadable.join("SKILL.md"), SKILL)?;
        fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o000))?;

        let loose = grant.join("src/loose");
        fs::create_dir_all(&loose)?;
        fs::write(loose.join("SKILL.md"), SKILL)?;
        Ok(())
    }

    struct Cleanup {
        base: PathBuf,
        locked: PathBuf,
    }

    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::set_permissions(&self.locked, fs::Permissions::from_mode(0o700));
            let _ = fs::remove_dir_all(&self.base);
        }
    }
}
