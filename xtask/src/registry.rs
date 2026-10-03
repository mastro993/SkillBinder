use std::{collections::BTreeMap, fmt::Write, path::Path};

use anyhow::{Context, ensure};
use serde::Deserialize;
use sha2::{Digest, Sha256};

const REGISTRY_SHA256: &str = "40a66c7e17208f84620c3abafc58f50d69ce95d914b7c2e0b67a26b24efc449d";
const UPSTREAM_COMMIT: &str = "7407f3893ad4dceab546ac002c3ef806e4000c73";
const UPSTREAM_FILES: [(&str, &str); 2] = [
    (
        "src/agents.ts",
        "8902f56a121ce4b1a800b3aa04d3f83b7baeaf32e1abf441c0b8eabb3f8f89a5",
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
    project_skills_dir: String,
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
        "The reviewed registry contains {} agents. Runtime discovery is not implemented in the native foundation.\n",
        registry.agents.len()
    )?;
    writeln!(
        text,
        "Source: `{}` at `{}`.\n",
        registry.upstream.repository, registry.upstream.commit
    )?;
    writeln!(
        text,
        "| Agent | ID | Project skills directory |\n| --- | --- | --- |"
    )?;
    for agent in registry.agents {
        writeln!(
            text,
            "| {} | `{}` | `{}` |",
            agent.display_name, agent.id, agent.project_skills_dir
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
