use super::*;
use crate::native_view::snapshot_query::fixture::Fixture;
use skillbinder_proto::{Preview, SkillSource, normalized_url};

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

#[test]
fn search_matches_name_description_creator_and_repository() {
    let catalog = catalog();
    assert_eq!(
        slugs(&results(&catalog, "pdf", None)),
        ["pdf-tools", "pdf-merge", "export"]
    );
    assert_eq!(
        slugs(&results(&catalog, "quill & co", None)),
        ["essay-coach"]
    );
    assert_eq!(slugs(&results(&catalog, "lumen/skills", None)).len(), 2);
    assert!(results(&catalog, "nothing like this", None).is_empty());
    // The repository matches by `owner/repo`, never by its host or scheme.
    assert!(results(&catalog, "example.invalid", None).is_empty());
    assert!(results(&catalog, "https", None).is_empty());
    let chips: Vec<_> = chips(&catalog, "pdf", None)
        .into_iter()
        .map(|(creator, n)| (creator.id.as_str(), n))
        .collect();
    assert_eq!(chips, [("northwind", 2), ("lumen", 1)]);
}

#[test]
fn a_picked_creator_narrows_the_results_and_keeps_its_chip() {
    let catalog = catalog();
    assert_eq!(slugs(&results(&catalog, "pdf", Some("lumen"))), ["export"]);
    assert_eq!(
        slugs(&results(&catalog, "", Some("northwind"))),
        ["pdf-tools", "pdf-merge"]
    );
    let picked: Vec<_> = chips(&catalog, "pdf", Some("quill"))
        .into_iter()
        .map(|(creator, n)| (creator.id.as_str(), n))
        .collect();
    assert_eq!(picked, [("quill", 0), ("northwind", 2), ("lumen", 1)]);
    let home: Vec<_> = chips(&catalog, "", Some("quill"))
        .into_iter()
        .map(|(creator, n)| (creator.id.as_str(), n))
        .collect();
    assert_eq!(home, [("lumen", 2), ("northwind", 2), ("quill", 1)]);
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
        assert_eq!(install_error(&pdf, &failure), line);
    }
}

#[test]
fn installs_read_as_a_short_gain() {
    assert_eq!(installs_label(0), "+0 this week");
    assert_eq!(installs_label(900), "+900 this week");
    assert_eq!(installs_label(1_000), "+1k this week");
    assert_eq!(installs_label(12_400), "+12.4k this week");
    assert_eq!(installs_label(48_249), "+48.2k this week");
    assert_eq!(installs_label(999_950), "+1M this week");
    assert_eq!(installs_label(2_350_000), "+2.4M this week");
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
