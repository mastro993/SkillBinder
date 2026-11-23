use super::{plan_install_all, problem, repositories, repository};
use crate::commands::explore::tests::{ACME, world};
use crate::commands::plan::{Plan, plan};
use crate::native_view::fixture::id;
use skillbinder_proto::RepositoryProblem;
use skillbinder_proto::command_line::{Change, ErrorCode};

#[test]
fn repositories_resolve_by_url_however_spelled_or_owner_repo() {
    let snapshot = world();
    for name in [
        ACME,
        "https://GitHub.com/Acme/Tools/",
        "acme/tools",
        "ACME/TOOLS",
    ] {
        assert_eq!(
            repository(&snapshot, name)
                .map(|found| found.id.0.as_str())
                .ok(),
            Some("acme"),
            "{name}"
        );
    }
    let rows = repositories(&snapshot);
    assert_eq!(
        rows.iter()
            .map(|row| (row.name.as_str(), row.skills, row.installable))
            .collect::<Vec<_>>(),
        [("acme/tools", 3, 2)],
        "docx is held, so Install all installs lint and pdf"
    );
}

#[test]
fn install_all_installs_what_the_library_lacks_or_nothing() {
    let mut snapshot = world();
    let Ok(Plan::InstallAll(repository)) = plan_install_all(&snapshot, "acme/tools") else {
        panic!("lint and pdf are not in library");
    };
    assert_eq!(repository.url, ACME);
    if let Some(library) = &mut snapshot.library {
        for slug in ["lint", "pdf"] {
            library.keys.insert(slug.into(), id(slug));
        }
    }
    let Ok(Plan::Save { saves, done }) = plan_install_all(&snapshot, "acme/tools") else {
        panic!("every slug is held");
    };
    assert!(saves.is_empty());
    assert_eq!(done, "Every skill of acme/tools is in your library.");
    let remove = Change::RemoveRepository {
        repository: "nope/nothing".into(),
    };
    assert_eq!(
        plan(&remove, &snapshot)
            .map(|_| ())
            .map_err(|error| error.code),
        Err(ErrorCode::NotFound)
    );
}

#[test]
fn repository_problems_read_as_the_add_repository_dialog_says() {
    let codes = [
        (RepositoryProblem::NotGitUrl, ErrorCode::Usage),
        (RepositoryProblem::Unreachable, ErrorCode::Failed),
        (RepositoryProblem::NoSkills, ErrorCode::Failed),
        (RepositoryProblem::AlreadyAdded, ErrorCode::Conflict),
        (RepositoryProblem::NeedsGit, ErrorCode::GitMissing),
    ];
    for (given, code) in codes {
        assert_eq!(problem(given.clone()).code, code, "{given:?}");
    }
    assert_eq!(
        problem(RepositoryProblem::Unreachable).message,
        "Couldn't reach this repository. Check the URL and your Git access."
    );
}
