//! skills.sh search and its results: the search answers from fixtures, and a result's folder is
//! found in its repository by the `skills` CLI's rules before it installs. Every repository is a
//! local one.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#[path = "support/publisher.rs"]
mod publisher;
#[path = "support/fixture.rs"]
mod support;

use publisher::Publisher;
use skillbinder_proto::{
    CatalogOrigin, ErrorCategory, InstallState, Resolution, SkillSource, SkillsShResult,
    SkillsShState, result_key,
};
use std::fs;
use support::{Fixture, category};

/// A repository at `<fixture>/<name>` holding `files`, committed.
fn repository(fixture: &Fixture, name: &str, files: &[(&str, &str)]) -> Publisher {
    let publisher = Publisher::new(fixture.workspace.with_file_name(name));
    for (path, content) in files {
        let path = publisher.dir.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    publisher.commit("skills");
    publisher
}

fn skill_md(name: &str) -> String {
    format!("---\nname: {name}\ndescription: Does {name} things\n---\n# {name}\n")
}

fn result(publisher: &Publisher, skill_id: &str) -> SkillsShResult {
    SkillsShResult {
        skill_id: skill_id.into(),
        key: skill_id.into(),
        name: skill_id.into(),
        url: publisher.url.clone(),
        installs: 1_234,
    }
}

#[tokio::test]
async fn the_search_answers_from_its_fixture_and_drops_older_queries() {
    let fixture = Fixture::new();
    let search = fixture.data.join("catalog-source/search");
    fs::create_dir_all(&search).unwrap();
    fs::write(
        search.join("pdf.json"),
        r#"{"skills": [{"source": "acme/skills", "skillId": "pdf", "name": "pdf", "installs": 7}]}"#,
    )
    .unwrap();
    let engine = fixture.open();
    let older = engine.search_skills_sh("docx");
    engine.search_skills_sh("pdf").await.unwrap();
    older.await.unwrap();

    let skills_sh = engine.current_explore().skills_sh;
    assert_eq!(skills_sh.query, "pdf");
    let SkillsShState::Results(results) = skills_sh.state else {
        panic!("no results: {:?}", skills_sh.state);
    };
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].url, "https://github.com/acme/skills.git");
    assert_eq!(results[0].installs, 7);

    // A query without an answer is unavailable, and one with no source needs a connection.
    engine.search_skills_sh("docx").await.unwrap();
    assert_eq!(
        engine.current_explore().skills_sh.state,
        SkillsShState::Unavailable
    );
    fs::remove_dir_all(&search).unwrap();
    engine.search_skills_sh("pdf").await.unwrap();
    assert_eq!(
        engine.current_explore().skills_sh.state,
        SkillsShState::NeedsConnection
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_result_resolves_to_its_folder_by_folder_name_or_skill_md_name() {
    let fixture = Fixture::new();
    let nested = repository(
        &fixture,
        "nested",
        &[
            ("README.md", "Skills"),
            ("skills/review/SKILL.md", &skill_md("code-review")),
            (".claude/skills/helper/SKILL.md", &skill_md("pdf-helper")),
            ("examples/deep/stray/SKILL.md", &skill_md("stray")),
        ],
    );
    let root = repository(&fixture, "solo", &[("SKILL.md", &skill_md("solo-notes"))]);
    let engine = fixture.ready().await;

    // The folder named `skillId` wins without reading its SKILL.md name.
    let review = engine
        .resolve(result(&nested, "review"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(review.slug, "code-review");
    assert_eq!(review.description, "Does code-review things");
    assert_eq!(review.origin, CatalogOrigin::SkillsSh { installs: 1_234 });
    assert_eq!(
        review.source,
        SkillSource {
            url: nested.url.clone(),
            reference: None,
            path: "skills/review".into(),
        }
    );
    // Else the folder whose SKILL.md name is `skillId`, in an agent's skill directory.
    let helper = engine
        .resolve(result(&nested, "pdf-helper"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(helper.source.path, ".claude/skills/helper");
    // A root skill is the repository's only one.
    let solo = engine
        .resolve(result(&root, "solo"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (solo.slug.as_str(), solo.source.path.as_str()),
        ("solo-notes", "")
    );
    // Outside the known places, a folder is not searched when the known places have skills.
    let stray = result(&nested, "stray");
    assert_eq!(engine.resolve(stray.clone()).await.unwrap(), None);

    let resolutions = engine.current_explore().resolutions;
    assert_eq!(
        resolutions[&result_key(&result(&nested, "review"))],
        Resolution::Resolved(review)
    );
    assert_eq!(resolutions[&result_key(&stray)], Resolution::NotFound);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn installing_a_result_resolves_it_first_and_a_missing_one_is_refused() {
    let fixture = Fixture::new();
    let repo = repository(
        &fixture,
        "tools",
        &[("skills/merge/SKILL.md", &skill_md("pdf-merge"))],
    );
    let engine = fixture.ready().await;

    let merge = result(&repo, "merge");
    engine.install_skills_sh(merge.clone()).await.unwrap();
    let snapshot = engine.snapshot().await.unwrap();
    let library = snapshot.library.unwrap();
    let installed = &library.library.skills[0];
    assert_eq!(installed.id.0, "pdf-merge");
    assert_eq!(
        installed.source.as_ref().unwrap().path,
        "skills/merge",
        "its source names the resolved folder"
    );
    assert!(installed.presets.is_empty());
    assert!(
        fixture
            .data
            .join("library/skills/pdf-merge/SKILL.md")
            .is_file()
    );
    assert!(snapshot.explore.installs.is_empty());

    let ghost = result(&repo, "ghost");
    assert_eq!(
        category(engine.install_skills_sh(ghost.clone()).await),
        ErrorCategory::Validation
    );
    let explore = engine.current_explore();
    assert_eq!(
        explore.resolutions[&result_key(&ghost)],
        Resolution::NotFound
    );
    assert_eq!(explore.installs.get(&result_key(&ghost)), None);

    // A repository that cannot be read keeps the failure on the result, to resolve again.
    let missing = SkillsShResult {
        url: url::Url::from_file_path(fixture.workspace.with_file_name("missing"))
            .unwrap()
            .into(),
        ..result(&repo, "gone")
    };
    assert_eq!(
        category(engine.install_skills_sh(missing.clone()).await),
        ErrorCategory::Git
    );
    let explore = engine.current_explore();
    assert_eq!(
        explore.resolutions[&result_key(&missing)],
        Resolution::Failed
    );
    assert!(matches!(
        explore.installs.get(&result_key(&missing)),
        Some(InstallState::Failed(_))
    ));
    engine.shutdown().await.unwrap();
}
