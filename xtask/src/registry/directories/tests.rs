#![expect(
    clippy::expect_used,
    reason = "Test assertions report fixture failures."
)]

use std::path::Path;

use super::*;

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
}

fn registry() -> Registry {
    super::super::load(root()).expect("reviewed registry")
}

fn fixture() -> (Registry, Overlay) {
    (registry(), load(root()).expect("reviewed overlay"))
}

fn entry<'a>(overlay: &'a mut Overlay, agent: &str) -> &'a mut Entry {
    overlay
        .parts
        .iter_mut()
        .flat_map(|part| part.agents.iter_mut())
        .find(|entry| entry.agent == agent)
        .expect("overlay entry")
}

fn part_of<'a>(overlay: &'a mut Overlay, agent: &str) -> &'a mut Part {
    overlay
        .parts
        .iter_mut()
        .find(|part| part.agents.iter().any(|entry| entry.agent == agent))
        .expect("overlay part")
}

fn rejected(registry: &Registry, overlay: &Overlay, rule: &str) {
    let error = check(registry, overlay).expect_err("overlay rule is enforced");
    assert!(error.to_string().contains(rule), "{error}");
}

/// Copies the overlay beside a temporary root, so `load` reads it from there.
fn copied() -> anyhow::Result<tempfile::TempDir> {
    let temp = tempfile::tempdir()?;
    let contracts = temp.path().join("fixtures/contracts");
    std::fs::create_dir_all(contracts.join("registry-directories"))?;
    for (path, _) in REGISTRY_DIRECTORIES_SHA256 {
        let name = Path::new(path).file_name().context("overlay file name")?;
        let target = temp.path().join(path);
        std::fs::copy(root().join(path), target.with_file_name(name))?;
    }
    Ok(temp)
}

#[test]
fn reviewed_overlay_matches_the_registry() -> anyhow::Result<()> {
    let registry = registry();
    check(&registry, &load(root())?)
}

#[test]
fn changed_bytes_are_rejected() -> anyhow::Result<()> {
    let temp = copied()?;
    let part = temp
        .path()
        .join("fixtures/contracts/registry-directories/k-z.json");
    let mut bytes = std::fs::read(&part)?;
    bytes.extend_from_slice(b" \n");
    std::fs::write(&part, bytes)?;
    let error = load(temp.path()).expect_err("changed bytes are rejected");
    assert!(error.to_string().contains("bytes changed"), "{error}");
    Ok(())
}

#[test]
fn changed_registry_sha_is_rejected() {
    let (registry, mut overlay) = fixture();
    overlay.registry_sha256 = "a".repeat(64);
    rejected(&registry, &overlay, "registrySha256");
}

#[test]
fn unknown_agent_is_rejected() {
    let (registry, mut overlay) = fixture();
    let mut unknown = entry(&mut overlay, "zed").clone();
    unknown.agent = "nope".into();
    part_of(&mut overlay, "zed").agents.push(unknown);
    rejected(&registry, &overlay, "outside the registry");
}

#[test]
fn repeated_agent_and_surface_is_rejected() {
    let (registry, mut overlay) = fixture();
    let repeated = entry(&mut overlay, "github-copilot").clone();
    part_of(&mut overlay, "github-copilot")
        .agents
        .push(repeated);
    rejected(&registry, &overlay, "repeats an agent and surface");
}

#[test]
fn out_of_order_entries_are_rejected() {
    let (registry, mut overlay) = fixture();
    overlay.parts[0].agents.swap(0, 1);
    rejected(&registry, &overlay, "lists agents out of order");
}

#[test]
fn entry_outside_its_part_range_is_rejected() {
    let (registry, mut overlay) = fixture();
    let codex = entry(&mut overlay, "codex").clone();
    for part in &mut overlay.parts {
        part.agents.retain(|entry| entry.agent != "codex");
    }
    overlay.parts[1].agents.insert(0, codex);
    rejected(&registry, &overlay, "outside the range its part holds");
}

#[test]
fn missing_registry_directory_is_rejected() {
    let (registry, mut overlay) = fixture();
    entry(&mut overlay, "zed")
        .workspace
        .retain(|directory| directory.path != ".agents/skills");
    rejected(
        &registry,
        &overlay,
        "omits the registry workspace directory",
    );

    let (registry, mut overlay) = fixture();
    entry(&mut overlay, "cursor")
        .global
        .retain(|directory| directory.path != "~/.cursor/skills");
    rejected(&registry, &overlay, "omits a registry global directory");
}

#[test]
fn repeated_path_after_expansion_is_rejected() {
    let (registry, mut overlay) = fixture();
    entry(&mut overlay, "amp").global.push(Directory {
        path: "~/.config/agents/skills".into(),
        role: Role::Shared,
    });
    rejected(&registry, &overlay, "lists a path twice after expansion");
}

#[test]
fn invalid_workspace_path_is_rejected() {
    for (path, rule) in [
        ("/absolute/skills", "is not relative"),
        ("../outside/skills", "leaves its directory"),
        (".", "resolves to nothing"),
        ("  ", "empty path"),
    ] {
        let (registry, mut overlay) = fixture();
        entry(&mut overlay, "zed").workspace.push(Directory {
            path: path.into(),
            role: Role::Compatibility,
        });
        rejected(&registry, &overlay, rule);
    }
}

#[test]
fn invalid_global_path_is_rejected() {
    let (registry, mut overlay) = fixture();
    entry(&mut overlay, "zed").global.push(Directory {
        path: "skills".into(),
        role: Role::Compatibility,
    });
    rejected(&registry, &overlay, "does not start with");
}

#[test]
fn shared_path_outside_the_shared_directories_is_rejected() {
    let (registry, mut overlay) = fixture();
    entry(&mut overlay, "zed").workspace.push(Directory {
        path: ".warp/skills".into(),
        role: Role::Shared,
    });
    rejected(&registry, &overlay, "outside the shared directories");
}

#[test]
fn unconfirmed_path_outside_the_registry_is_rejected() {
    let (registry, mut overlay) = fixture();
    entry(&mut overlay, "zed").global.push(Directory {
        path: "~/.warp/skills".into(),
        role: Role::Unconfirmed,
    });
    rejected(
        &registry,
        &overlay,
        "no registry entry lists as unconfirmed",
    );
}

#[test]
fn order_naming_an_unlisted_path_is_rejected() {
    let (registry, mut overlay) = fixture();
    let order = entry(&mut overlay, "zed")
        .order
        .as_mut()
        .expect("zed order");
    order.push(vec![".nope/skills".into()]);
    rejected(&registry, &overlay, "names an unlisted path");
}

#[test]
fn order_rules_are_enforced() {
    let (registry, mut overlay) = fixture();
    entry(&mut overlay, "zed").order = None;
    rejected(&registry, &overlay, "needs an order");

    let (registry, mut overlay) = fixture();
    let paths = entry(&mut overlay, "warp");
    paths.order = Some(
        paths
            .workspace
            .iter()
            .chain(paths.global.iter())
            .map(|directory| vec![directory.path.clone()])
            .collect(),
    );
    rejected(&registry, &overlay, "gives an order for an undecided");

    let (registry, mut overlay) = fixture();
    entry(&mut overlay, "zed").order = Some(vec![vec![]]);
    rejected(&registry, &overlay, "empty order tier");

    let (registry, mut overlay) = fixture();
    entry(&mut overlay, "zed").order = Some(vec![
        vec![".agents/skills".into()],
        vec![".agents/skills".into()],
    ]);
    rejected(&registry, &overlay, "names a path twice");

    let (registry, mut overlay) = fixture();
    entry(&mut overlay, "zed").order = Some(vec![vec![".agents/skills".into()]]);
    rejected(&registry, &overlay, "leaves listed paths unranked");
}

#[test]
fn source_without_its_pin_is_rejected() {
    let (registry, mut overlay) = fixture();
    entry(&mut overlay, "cline").sources[0].revision = None;
    rejected(&registry, &overlay, "has no revision");

    let (registry, mut overlay) = fixture();
    entry(&mut overlay, "cline").sources[0].revision = Some("v4.1.26".into());
    rejected(&registry, &overlay, "is not a full commit SHA");

    let (registry, mut overlay) = fixture();
    entry(&mut overlay, "cursor").sources[0].retrieved_at = None;
    rejected(&registry, &overlay, "has no retrievedAt date");

    let (registry, mut overlay) = fixture();
    entry(&mut overlay, "cursor").sources[0].retrieved_at = Some("2026-11".into());
    rejected(&registry, &overlay, "retrievedAt is not a date");

    let (registry, mut overlay) = fixture();
    entry(&mut overlay, "zed").sources[0].url = "github.com/zed-industries/zed".into();
    rejected(&registry, &overlay, "is not an https URL");

    let (registry, mut overlay) = fixture();
    entry(&mut overlay, "zed").sources.clear();
    rejected(&registry, &overlay, "has no source");
}

#[test]
fn other_directories_leave_out_the_registry_ones() -> anyhow::Result<()> {
    let registry = registry();
    let overlay = load(root())?;
    let warp = registry
        .agents
        .iter()
        .find(|agent| agent.id == "warp")
        .context("warp")?;
    let others = others(&overlay, warp);
    assert!(others.contains(&".warp/skills".to_owned()));
    assert!(!others.contains(&".agents/skills".to_owned()));
    assert!(!others.contains(&"~/.agents/skills".to_owned()));
    Ok(())
}
