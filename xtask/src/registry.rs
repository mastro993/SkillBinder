use std::{collections::BTreeMap, fmt::Write, path::Path};

use anyhow::{Context, ensure};
use serde::Deserialize;
use sha2::{Digest, Sha256};

const REGISTRY_SHA256: &str = "eb1365a541f5e95f3adc6bb5f3c0a1a2fb2f84be0d82ec942f50b669b004c044";
const UPSTREAM_COMMIT: &str = "13e4063a1cf913f5606d57d42ab83a86f5001e04";
const UPSTREAM_FILES: [(&str, &str); 2] = [
    (
        "src/agents.ts",
        "9c64197246db1b61c0b2777114e2e01dd60f503129842ce7eeb78a3c3f2caa25",
    ),
    (
        "src/types.ts",
        "0f444746b0ef0da541e23184f401eb0da81b3abad217c934e89cc54970c84d9c",
    ),
];

#[derive(Deserialize)]
struct Registry {
    upstream: Upstream,
    agents: Vec<Agent>,
}

#[derive(Deserialize)]
struct Upstream {
    repository: String,
    commit: String,
    files: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Agent {
    id: String,
    display_name: String,
    /// The upstream registry calls a workspace a project.
    #[serde(rename = "projectSkillsDir")]
    workspace_skills_dir: String,
}

fn load(root: &Path) -> anyhow::Result<Registry> {
    let bytes = std::fs::read(root.join("fixtures/contracts/registry-79.json"))?;
    ensure!(
        format!("{:x}", Sha256::digest(&bytes)) == REGISTRY_SHA256,
        "Reviewed registry bytes changed; a new provenance review is required"
    );
    let registry: Registry = serde_json::from_slice(&bytes).context("Invalid agent registry")?;
    ensure!(
        registry.upstream.repository == "vercel-labs/skills"
            && registry.upstream.commit == UPSTREAM_COMMIT,
        "Registry upstream revision changed"
    );
    ensure!(
        registry.upstream.files.len() == UPSTREAM_FILES.len()
            && UPSTREAM_FILES.iter().all(|(path, hash)| registry
                .upstream
                .files
                .get(*path)
                .is_some_and(|actual| actual == hash)),
        "Registry source provenance changed"
    );
    Ok(registry)
}

fn document(mut registry: Registry) -> anyhow::Result<String> {
    registry.agents.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
    });
    let mut text = String::from("# Supported agent registry\n\n");
    writeln!(
        text,
        "The reviewed registry contains {} agents. Installed agents are the agents whose global skill directory exists on this device; entries are not runtime-tested.\n",
        registry.agents.len()
    )?;
    writeln!(
        text,
        "Source: `{}` at `{}`.\n",
        registry.upstream.repository, registry.upstream.commit
    )?;
    writeln!(
        text,
        "| Agent | ID | Workspace skills directory |\n| --- | --- | --- |"
    )?;
    for agent in registry.agents {
        writeln!(
            text,
            "| {} | `{}` | `{}` |",
            agent.display_name, agent.id, agent.workspace_skills_dir
        )?;
    }
    Ok(text)
}

pub fn verify(root: &Path) -> anyhow::Result<()> {
    let expected = document(load(root)?)?;
    let actual = std::fs::read_to_string(root.join("docs/supported-agents.md"))?;
    ensure!(
        actual.replace("\r\n", "\n") == expected,
        "Registry documentation is stale; run cargo run -p xtask -- registry > docs/supported-agents.md"
    );
    Ok(())
}

pub fn print(root: &Path) -> anyhow::Result<()> {
    print!("{}", document(load(root)?)?);
    Ok(())
}
