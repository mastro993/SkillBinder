use super::*;
use crate::native_view::snapshot_query::fixture::Fixture;
use skillbinder_proto::{
    CatalogState, InstallAllOutcome, Preview, Repository, RepositoryId, Resolution, SkillSource,
    SkillsShResult, SkillsShSearch, SkillsShState, normalized_url, result_key,
};

fn creator(id: &str, name: &str) -> Creator {
    Creator {
        id: id.to_owned(),
        name: name.to_owned(),
        avatar: None,
    }
}

fn skill(slug: &str, creator: &str, description: &str, installs: u64) -> CatalogSkill {
    CatalogSkill {
        slug: slug.to_owned(),
        key: slug.to_owned(),
        name: slug.to_owned(),
        description: description.to_owned(),
        origin: CatalogOrigin::Curated {
            creator: creator.to_owned(),
        },
        source: SkillSource {
            url: format!("https://git.example.invalid/{creator}/skills.git"),
            reference: None,
            path: format!("skills/{slug}"),
        },
        installs_week: installs,
    }
}

/// Northwind publishes two PDF skills; Lumen one popular design skill and one whose description
/// mentions PDF; Quill one quiet skill.
fn catalog() -> Catalog {
    Catalog {
        creators: vec![
            creator("northwind", "Northwind Labs"),
            creator("lumen", "Lumen Studio"),
            creator("quill", "Quill & Co"),
        ],
        skills: vec![
            skill("ui-review", "lumen", "Reviews interfaces.", 48_200),
            skill("pdf-tools", "northwind", "Fills forms.", 12_400),
            skill("pdf-merge", "northwind", "Merges files.", 8_300),
            skill("export", "lumen", "Exports a PDF report.", 2_100),
            skill("essay-coach", "quill", "Gives feedback.", 1_200),
        ],
    }
}

fn slugs(skills: &[&CatalogSkill]) -> Vec<String> {
    skills.iter().map(|skill| skill.slug.clone()).collect()
}

#[test]
fn trending_takes_the_six_most_installed_and_creators_rank_by_their_installs() {
    let catalog = catalog();
    assert_eq!(trending(&catalog).len(), 5);
    let ranked: Vec<_> = popular_creators(&catalog)
        .iter()
        .map(|ranked| (ranked.creator.id.as_str(), ranked.skills, ranked.installs))
        .collect();
    assert_eq!(
        ranked,
        [
            ("lumen", 2, 50_300),
            ("northwind", 2, 20_700),
            ("quill", 1, 1_200)
        ]
    );
}

/// Your repository `id` at `url`, listing each of `slugs` under `skills/`.
fn repository(id: &str, url: &str, slugs: &[&str]) -> Repository {
    let id = RepositoryId(id.to_owned());
    Repository {
        skills: slugs
            .iter()
            .map(|slug| CatalogSkill {
                origin: CatalogOrigin::Repository { id: id.clone() },
                source: SkillSource {
                    url: url.to_owned(),
                    reference: None,
                    path: format!("skills/{slug}"),
                },
                installs_week: 0,
                ..skill(slug, "", "From a repository.", 0)
            })
            .collect(),
        id,
        url: url.to_owned(),
        listed_at: 0,
        added_at: 0,
    }
}

fn chip_names(chips: &[(Chip<'_>, usize)]) -> Vec<(String, usize)> {
    chips
        .iter()
        .map(|(chip, n)| {
            let name = match chip {
                Chip::Creator(creator) => creator.id.clone(),
                Chip::Repository(repository) => owner_repo(&repository.url),
            };
            (name, *n)
        })
        .collect()
}

#[test]
fn search_matches_name_description_creator_and_repository() {
    let catalog = catalog();
    let local = Local {
        catalog: &catalog,
        repositories: &[],
    };
    assert_eq!(
        slugs(&local.results("pdf", None)),
        ["pdf-tools", "pdf-merge", "export"]
    );
    assert_eq!(slugs(&local.results("quill & co", None)), ["essay-coach"]);
    assert_eq!(slugs(&local.results("lumen/skills", None)).len(), 2);
    assert!(local.results("nothing like this", None).is_empty());
    // The repository matches by `owner/repo`, never by its host or scheme.
    assert!(local.results("example.invalid", None).is_empty());
    assert!(local.results("https", None).is_empty());
    assert_eq!(
        chip_names(&local.chips("pdf", None)),
        [("northwind".to_owned(), 2), ("lumen".to_owned(), 1)]
    );
}

#[test]
fn a_picked_creator_narrows_the_results_and_keeps_its_chip() {
    let catalog = catalog();
    let local = Local {
        catalog: &catalog,
        repositories: &[],
    };
    let creator = |id: &str| Source::Creator(id.to_owned());
    assert_eq!(
        slugs(&local.results("pdf", Some(&creator("lumen")))),
        ["export"]
    );
    assert_eq!(
        slugs(&local.results("", Some(&creator("northwind")))),
        ["pdf-tools", "pdf-merge"]
    );
    assert_eq!(
        chip_names(&local.chips("pdf", Some(&creator("quill")))),
        [
            ("quill".to_owned(), 0),
            ("northwind".to_owned(), 2),
            ("lumen".to_owned(), 1)
        ]
    );
    assert_eq!(
        chip_names(&local.chips("", Some(&creator("quill")))),
        [
            ("lumen".to_owned(), 2),
            ("northwind".to_owned(), 2),
            ("quill".to_owned(), 1)
        ]
    );
}

#[test]
fn your_repositories_follow_the_catalog_by_name_and_a_curated_skill_shows_once() {
    // Northwind's own repository also lists pdf-tools, which the catalog has.
    let northwind = repository(
        "northwind",
        "https://git.example.invalid/northwind/skills.git",
        &["pdf-tools", "pdf-split"],
    );
    let private = repository(
        "private",
        "git@git.example.invalid:acme/private.git",
        &["zeta-pdf", "alpha-pdf"],
    );
    let catalog = catalog();
    let repositories = [northwind.clone(), private.clone()];
    let local = Local {
        catalog: &catalog,
        repositories: &repositories,
    };
    assert_eq!(
        slugs(&local.results("pdf", None)),
        [
            "pdf-tools",
            "pdf-merge",
            "export",
            "alpha-pdf",
            "pdf-split",
            "zeta-pdf"
        ]
    );
    // The curated duplicate counts under its repository's chip too.
    assert_eq!(
        chip_names(&local.chips("pdf", None)),
        [
            ("northwind".to_owned(), 2),
            ("lumen".to_owned(), 1),
            ("northwind/skills".to_owned(), 2),
            ("acme/private".to_owned(), 2)
        ]
    );
    let picked = Source::Repository(private.id.clone());
    assert_eq!(
        slugs(&local.results("", Some(&picked))),
        ["alpha-pdf", "zeta-pdf"]
    );
    assert_eq!(local.repository_of(&private.skills[0]), Some(&private));
    assert_eq!(repository_glyph(&private.url), "lock");
    assert_eq!(repository_glyph(&northwind.url), "github");
}

#[test]
fn install_all_counts_the_slugs_the_library_lacks_and_sums_up_once() {
    let repository = repository(
        "acme",
        "https://git.example.invalid/acme/skills.git",
        &["pdf-tools", "docx", "blocked"],
    );
    // The library holds pdf-tools from elsewhere, so Install all leaves it.
    let snapshot = Fixture::new().skill("pdf-tools", "", None).build();
    assert_eq!(installable(&snapshot, &repository), 2);

    let mut outcome = InstallAllOutcome {
        installed: vec![SkillId("docx".to_owned())],
        failed: Vec::new(),
    };
    assert_eq!(summary(&outcome), "Installed 1");
    assert_eq!(toast(&outcome), "Installed 1 skill");
    outcome.installed.push(SkillId("pdf-tools".to_owned()));
    assert_eq!(toast(&outcome), "Installed 2 skills");
    outcome
        .failed
        .push(("blocked".to_owned(), InstallFailure::Blocked));
    assert_eq!(summary(&outcome), "Installed 2 · 1 failed");
    assert_eq!(toast(&outcome), "Installed 2 · 1 failed");
    assert_eq!(
        failures(&repository, &outcome),
        "Couldn't install blocked: it is blocked."
    );
}

#[test]
fn a_held_slug_is_in_library_and_new_only_from_the_same_source() {
    let pdf = skill("pdf-tools", "northwind", "", 1);
    let mut snapshot = Fixture::new().skill("pdf-tools", "", None).build();
    assert!(!in_library(&snapshot, &pdf), "a skill with no source");
    let library = snapshot.library.as_mut().unwrap_or_else(|| unreachable!());
    let held = &mut library.library.skills[0];
    held.source = Some(SkillSource {
        url: "https://git.example.invalid/northwind/skills".to_owned(),
        reference: Some("main".to_owned()),
        path: "skills/pdf-tools".to_owned(),
    });
    assert!(in_library(&snapshot, &pdf));
    assert!(!collides(&snapshot, &pdf));
    assert_eq!(new_in_library(&snapshot, &pdf), None, "not New");
    let library = snapshot.library.as_mut().unwrap_or_else(|| unreachable!());
    library.library.skills[0].new = true;
    assert_eq!(
        new_in_library(&snapshot, &pdf),
        Some(&SkillId("pdf-tools".to_owned()))
    );
    let mut moved = pdf.clone();
    moved.source.url = "git@git.example.invalid:northwind/skills.git".to_owned();
    assert_ne!(
        normalized_url(&moved.source.url),
        normalized_url(&pdf.source.url)
    );
    assert!(!in_library(&snapshot, &moved), "another source");
    assert_eq!(new_in_library(&snapshot, &moved), None);
    assert!(
        collides(&snapshot, &moved),
        "Install is refused, never renamed"
    );
    let docx = skill("docx", "northwind", "", 1);
    assert!(!in_library(&snapshot, &docx));
    assert!(!collides(&snapshot, &docx));
}

#[test]
fn an_install_is_found_by_its_source_and_its_failure_reads_as_one_line() {
    let pdf = skill("pdf-tools", "northwind", "", 1);
    let mut snapshot = Fixture::new().build();
    assert_eq!(install_state(&snapshot, &pdf), None);
    let mut spelled = pdf.source.clone();
    spelled.path.push('/');
    snapshot
        .explore
        .installs
        .insert(install_key(&spelled), InstallState::Installing);
    assert_eq!(
        install_state(&snapshot, &pdf),
        Some(&InstallState::Installing)
    );
    for (failure, line) in [
        (
            InstallFailure::Access,
            "Couldn't install. Check your access to northwind/skills.",
        ),
        (InstallFailure::Offline, "Couldn't install: no connection."),
        (
            InstallFailure::NotFound,
            "Couldn't find this skill in northwind/skills",
        ),
        (
            InstallFailure::Blocked,
            "Couldn't install pdf-tools: it is blocked.",
        ),
        (
            InstallFailure::Invalid,
            "Couldn't install pdf-tools: its SKILL.md metadata is invalid.",
        ),
        (
            InstallFailure::Empty,
            "Couldn't install pdf-tools: its folder has no files.",
        ),
    ] {
        assert_eq!(install_error(&pdf.slug, &pdf.source.url, &failure), line);
    }
}

#[test]
fn installs_read_as_a_short_gain_or_all_time_on_skills_sh_and_none_from_a_repository() {
    let label = |installs| installs_label(&skill("pdf", "northwind", "", installs));
    assert_eq!(label(0).as_deref(), Some("+0 this week"));
    assert_eq!(label(900).as_deref(), Some("+900 this week"));
    assert_eq!(label(1_000).as_deref(), Some("+1k this week"));
    assert_eq!(label(12_400).as_deref(), Some("+12.4k this week"));
    assert_eq!(label(48_249).as_deref(), Some("+48.2k this week"));
    assert_eq!(label(999_950).as_deref(), Some("+1M this week"));
    assert_eq!(label(2_350_000).as_deref(), Some("+2.4M this week"));
    let mut found = skill("pdf", "northwind", "", 0);
    found.origin = CatalogOrigin::SkillsSh { installs: 233_102 };
    assert_eq!(installs_label(&found).as_deref(), Some("233.1k installs"));
    // Your repositories have no install counts.
    found.origin = CatalogOrigin::Repository {
        id: RepositoryId("acme".to_owned()),
    };
    assert_eq!(installs_label(&found), None);
    assert_eq!(all_time_label(7), "7 installs");
}

#[test]
fn a_slug_repeated_across_repositories_keys_its_elements_apart() {
    let northwind = skill("pdf", "northwind", "", 1);
    let quill = skill("pdf", "quill", "", 1);
    assert_ne!(element_key(&northwind), element_key(&quill));
    let mut spelled = northwind.clone();
    spelled.source.url = "https://GIT.example.invalid/northwind/skills".to_owned();
    assert_eq!(element_key(&spelled), element_key(&northwind));
}

#[test]
fn a_preview_belongs_to_the_sheet_of_its_source() {
    let mut snapshot = Fixture::new().build();
    let pdf = skill("pdf", "northwind", "", 1);
    assert_eq!(preview_state(&snapshot, &pdf), None);
    let mut spelled = pdf.source.clone();
    spelled.path.push('/');
    snapshot.explore.preview = Some(Preview {
        source: spelled,
        state: PreviewState::Failed,
    });
    assert_eq!(preview_state(&snapshot, &pdf), Some(&PreviewState::Failed));
    assert_eq!(
        preview_state(&snapshot, &skill("pdf", "quill", "", 1)),
        None
    );
}

#[test]
fn open_in_browser_maps_every_url_to_the_repository_web_page() {
    let page = |url: &str, path: &str| {
        browser_url(&SkillSource {
            url: url.to_owned(),
            reference: None,
            path: path.to_owned(),
        })
    };
    let pdf = Some("https://github.com/acme/skills/tree/HEAD/skills/pdf".to_owned());
    for url in [
        "https://github.com/Acme/Skills.git",
        "git@github.com:acme/skills.git",
        "ssh://git@github.com/acme/skills",
    ] {
        assert_eq!(page(url, "skills/pdf/"), pdf, "{url}");
    }
    assert_eq!(
        page("https://github.com/acme/skills.git", ""),
        Some("https://github.com/acme/skills".to_owned())
    );
    // Other hosts get the repository's page; an SSH port is dropped.
    assert_eq!(
        page("ssh://git@gitlab.com:2222/Team/Repo.git", "skills/pdf"),
        Some("https://gitlab.com/Team/Repo".to_owned())
    );
    assert_eq!(page("file:///tmp/skills", "skills/pdf"), None);
}

#[test]
fn front_matter_keys_read_as_labels() {
    for (key, label) in [
        ("license", "License"),
        ("allowed-tools", "Allowed tools"),
        ("argument_hint", "Argument hint"),
        ("", ""),
    ] {
        assert_eq!(front_matter_label(key), label);
    }
}

#[test]
fn front_matter_is_dropped_from_skill_md() {
    assert_eq!(
        without_front_matter("---\nname: pdf\n---\n# PDF\n"),
        "# PDF\n"
    );
    assert_eq!(
        without_front_matter("---\r\nname: pdf\r\n---\r\nBody"),
        "Body"
    );
    // Without a closing line, or without front matter, the text stays whole.
    for text in ["---\nname: pdf\n", "# PDF\n---\n"] {
        assert_eq!(without_front_matter(text), text);
    }
}

fn found_result(skill_id: &str, url: &str) -> SkillsShResult {
    SkillsShResult {
        skill_id: skill_id.to_owned(),
        key: skill_id.to_owned(),
        name: skill_id.to_owned(),
        url: url.to_owned(),
        installs: 10,
    }
}

#[test]
fn skills_sh_answers_only_the_search_shown_after_two_characters() {
    let catalog = catalog();
    let local = Local {
        catalog: &catalog,
        repositories: &[],
    };
    let mut snapshot = Fixture::new().build();
    snapshot.explore.state = CatalogState::Ready;
    assert_eq!(remote(&snapshot, local, "p"), Remote::Idle);
    // Until the pause ends and the request answers, it is on its way.
    assert_eq!(remote(&snapshot, local, "pdf"), Remote::Searching);
    snapshot.explore.skills_sh = SkillsShSearch {
        query: "pd".to_owned(),
        state: SkillsShState::Unavailable,
    };
    assert_eq!(remote(&snapshot, local, "pdf"), Remote::Searching);
    assert_eq!(remote(&snapshot, local, "pd"), Remote::Unavailable);
    snapshot.explore.skills_sh.state = SkillsShState::NeedsConnection;
    assert_eq!(remote(&snapshot, local, "pd"), Remote::NeedsConnection);
    // An offline catalog means skills.sh needs a connection too.
    snapshot.explore.skills_sh.state = SkillsShState::Results(Vec::new());
    snapshot.explore.state = CatalogState::Offline;
    assert_eq!(remote(&snapshot, local, "pd"), Remote::NeedsConnection);
    snapshot.explore.state = CatalogState::Ready;
    assert_eq!(remote(&snapshot, local, "pd").count(), Some(0));
}

#[test]
fn skills_sh_results_leave_out_skills_shown_locally_and_show_resolved_ones_as_skills() {
    let catalog = catalog();
    // One of your repositories lists the result skills.sh also has.
    let yours = found_result("pdf-notes", "https://git.example.invalid/acme/team.git");
    let repositories = [repository("team", &yours.url, &["pdf-notes"])];
    let local = Local {
        catalog: &catalog,
        repositories: &repositories,
    };
    let shown = found_result("pdf-tools", "https://git.example.invalid/northwind/skills/");
    let other = found_result("pdf-tools", "https://github.com/acme/skills.git");
    let renamed = found_result("merge", "https://git.example.invalid/northwind/skills.git");
    let mut snapshot = Fixture::new().build();
    snapshot.explore.state = CatalogState::Ready;
    snapshot.explore.skills_sh = SkillsShSearch {
        query: "pdf".to_owned(),
        state: SkillsShState::Results(vec![shown, other.clone(), renamed.clone(), yours]),
    };
    let Remote::Results(found) = remote(&snapshot, local, "pdf") else {
        panic!("no results");
    };
    assert_eq!(found, [Found::Result(&other), Found::Result(&renamed)]);

    // Once resolved, a result is its skill, and one the catalog shows is left out by its slug.
    let mut merge = skill("pdf-merge", "northwind", "", 0);
    merge.origin = CatalogOrigin::SkillsSh { installs: 10 };
    snapshot
        .explore
        .resolutions
        .insert(result_key(&renamed), Resolution::Resolved(Box::new(merge)));
    let mut resolved = skill("pdf-tools", "acme", "", 0);
    resolved.source.url = other.url.clone();
    snapshot.explore.resolutions.insert(
        result_key(&other),
        Resolution::Resolved(Box::new(resolved.clone())),
    );
    assert_eq!(
        remote(&snapshot, local, "pdf"),
        Remote::Results(vec![Found::Skill(&resolved)])
    );
}

#[test]
fn an_unresolved_result_is_in_library_from_its_repository_and_collides_from_another() {
    let result = found_result("pdf-tools", "https://github.com/acme/skills.git");
    let mut snapshot = Fixture::new().skill("pdf-tools", "", None).build();
    assert!(!result_in_library(&snapshot, &result));
    assert!(
        result_collides(&snapshot, &result),
        "a skill with no source"
    );
    let library = snapshot.library.as_mut().unwrap_or_else(|| unreachable!());
    library.library.skills[0].source = Some(SkillSource {
        url: "https://github.com/Acme/Skills".to_owned(),
        reference: None,
        path: "skills/pdf".to_owned(),
    });
    assert!(result_in_library(&snapshot, &result), "any folder of it");
    assert!(!result_collides(&snapshot, &result));
    let elsewhere = found_result("pdf-tools", "https://github.com/other/skills.git");
    assert!(result_collides(&snapshot, &elsewhere));
}
