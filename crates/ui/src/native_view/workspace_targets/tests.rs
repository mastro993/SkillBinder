use super::*;
use skillbinder_proto::{AgentDir, SameSlug};

fn agent(id: &str, name: &str, dir: &str, detected: bool) -> AgentOption {
    AgentOption {
        id: id.to_owned(),
        display_name: name.to_owned(),
        workspace_skills_dir: dir.to_owned(),
        other_workspace_dirs: Vec::new(),
        global_skills_dir: Some(format!("/home/{id}")),
        resolved_skills_dir: Some(format!("/home/{id}")),
        other_global_dirs: Vec::new(),
        same_slug: SameSlug::Unknown,
        detected,
    }
}

fn workspace(dirs: &[&str]) -> Workspace {
    Workspace {
        id: WorkspaceId("w".to_owned()),
        name: "api".to_owned(),
        path: "/home/api".to_owned(),
        position: 0,
        missing: false,
        color: None,
        icon: None,
        image: None,
        dirs: set(dirs),
        paths: dirs
            .iter()
            .map(|dir| ((*dir).to_owned(), format!("/resolved/{dir}")))
            .collect(),
        skills: Default::default(),
        overlaps: Default::default(),
    }
}

fn set(dirs: &[&str]) -> BTreeSet<String> {
    dirs.iter().map(|dir| (*dir).to_owned()).collect()
}

#[test]
fn rows_list_each_directory_once_naming_the_agents_that_read_it() {
    let mut codex = agent("codex", "Codex", ".agents/skills", true);
    codex.global_skills_dir = Some("/home/.agents/skills".to_owned());
    let agents = [
        codex,
        agent("cursor", "Cursor", ".agents/skills", true),
        agent("amp", "Amp", ".agents/skills", false),
        agent("zed", "Zed", ".zed/skills", true),
        agent("qoder", "Qoder", ".qoder/skills", false),
        agent("qoder-cn", "Qoder CN", ".qoder/skills", false),
        agent("windsurf", "Windsurf", ".windsurf/skills", false),
    ];
    let rows = dir_rows(&agents, &workspace(&[".qoder/skills"]), "/home");
    let shown: Vec<_> = rows
        .iter()
        .map(|row| (row.dir.as_str(), row.row.name.as_str()))
        .collect();
    assert_eq!(
        shown,
        [
            (".agents/skills", "Universal"),
            (".qoder/skills", "Qoder, Qoder CN"),
            (".zed/skills", "Zed"),
        ],
        "undetected agents count only for directories the workspace uses"
    );
    assert_eq!(
        rows[0].row.reader,
        Reader::Universal(vec!["Codex".to_owned(), "Cursor".to_owned()])
    );
    assert_eq!(rows[2].row.dir, "~/api/.zed/skills");
    assert_eq!(rows[2].row.resolved, "/home/api/.zed/skills");
    assert_eq!(
        rows[1].row.resolved, "/resolved/.qoder/skills",
        "a directory the workspace uses matches its copies' resolved path"
    );
}

#[test]
fn rows_offer_a_detected_agents_own_directory_and_name_every_reader() {
    let mut kimi = agent("kimi-code-cli", "Kimi Code CLI", ".agents/skills", true);
    kimi.other_workspace_dirs = [AgentDir {
        path: ".kimi-code/skills".to_owned(),
        role: DirRole::Own,
    }]
    .into();
    let mut cursor = agent("cursor", "Cursor", ".agents/skills", true);
    cursor.other_workspace_dirs = [
        AgentDir {
            path: ".claude/skills".to_owned(),
            role: DirRole::Compatibility,
        },
        AgentDir {
            path: ".cursor/skills".to_owned(),
            role: DirRole::Own,
        },
    ]
    .into();
    let agents = [
        kimi,
        cursor,
        agent("claude-code", "Claude Code", ".claude/skills", true),
    ];
    let rows = dir_rows(&agents, &workspace(&[".claude/skills"]), "/home");
    let shown: Vec<_> = rows
        .iter()
        .map(|row| (row.dir.as_str(), row.row.name.as_str()))
        .collect();
    assert_eq!(
        shown,
        [
            (".agents/skills", "Kimi Code CLI, Cursor"),
            (".cursor/skills", "Cursor"),
            (".claude/skills", "Cursor, Claude Code"),
            (".kimi-code/skills", "Kimi Code CLI"),
        ],
        "an own directory is offered; another agent's only once the workspace uses it"
    );
    assert_eq!(rows[2].row.ids, ["cursor", "claude-code"]);
}

#[test]
fn two_directories_of_one_agent_name_their_own_rows() {
    let mut cline = agent("cline", "Cline", ".agents/skills", true);
    cline.other_workspace_dirs = [
        AgentDir {
            path: ".cline/skills".to_owned(),
            role: DirRole::Own,
        },
        AgentDir {
            path: ".clinerules/skills".to_owned(),
            role: DirRole::Own,
        },
    ]
    .into();
    let rows = dir_rows(&[cline], &workspace(&[]), "/home");
    let shown: Vec<_> = rows
        .iter()
        .map(|row| (row.dir.as_str(), row.row.name.as_str()))
        .collect();
    assert_eq!(
        shown,
        [
            (".agents/skills", "Cline"),
            (".cline/skills", "Cline (.cline/skills)"),
            (".clinerules/skills", "Cline (.clinerules/skills)"),
        ]
    );
}

#[test]
fn choosing_a_row_toggles_its_directory_into_a_specific_list() {
    let uses = set(&[".agents/skills", ".claude/skills"]);
    let all = Some(&WorkspaceTargets::AllAgents);
    let dirs = |dirs: &[&str]| Some(WorkspaceTargets::Dirs(set(dirs)));
    let one = |dir: &str| vec![dir.to_owned()];
    assert_eq!(
        toggled(all, &one(".claude/skills"), &uses),
        dirs(&[".agents/skills"])
    );
    assert_eq!(
        toggled(all, &one(".zed/skills"), &uses),
        dirs(&[".agents/skills", ".claude/skills", ".zed/skills"]),
        "a new directory joins with every one the workspace uses"
    );
    assert_eq!(
        toggled(None, &one(".zed/skills"), &uses),
        dirs(&[".zed/skills"])
    );
    let zed = WorkspaceTargets::Dirs(set(&[".zed/skills"]));
    assert_eq!(toggled(Some(&zed), &one(".zed/skills"), &uses), None);
    let linked = vec![".agents/skills".to_owned(), ".claude/skills".to_owned()];
    assert_eq!(
        toggled(dirs(&[".claude/skills"]).as_ref(), &linked, &uses),
        None,
        "directories sharing a folder leave together"
    );
}

#[test]
fn rows_sharing_a_folder_are_chosen_together_and_universal_needs_a_universal_agent() {
    let mut ws = workspace(&[".agents/skills", ".claude/skills"]);
    ws.paths.insert(
        ".claude/skills".to_owned(),
        "/resolved/.agents/skills".to_owned(),
    );
    let agents = [
        agent("cursor", "Cursor", ".agents/skills", true),
        agent("claude-code", "Claude Code", ".claude/skills", true),
    ];
    let rows = dir_rows(&agents, &ws, "/home");
    assert_eq!(
        sharing(&rows[1], &rows),
        [".agents/skills".to_owned(), ".claude/skills".to_owned()]
    );
    assert_eq!(
        (rows[0].row.name.as_str(), &rows[0].row.reader),
        ("Cursor", &Reader::Agent("cursor".to_owned())),
        "only Cursor reads it here, and Cursor's page is its own"
    );
}

#[test]
fn the_label_names_all_agents_one_row_or_a_count() {
    let agents = [
        agent("codex", "Codex", ".agents/skills", true),
        agent("claude-code", "Claude Code", ".claude/skills", true),
    ];
    let rows = dir_rows(&agents, &workspace(&[]), "/home");
    let dirs = |dirs: &[&str]| WorkspaceTargets::Dirs(set(dirs));
    assert_eq!(targets_label(None, &rows), "All agents");
    assert_eq!(
        targets_label(Some(&dirs(&[".claude/skills"])), &rows),
        "Claude Code"
    );
    let both = dirs(&[".agents/skills", ".claude/skills"]);
    assert_eq!(targets_label(Some(&both), &rows), "2 agents");
    assert_eq!(
        targets_label(Some(&dirs(&[".gone/skills"])), &rows),
        "No agents"
    );
}
