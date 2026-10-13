use std::path::{Component, Path};

use anyhow::{Context, ensure};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Baseline {
    baseline_revision: String,
    image_count: usize,
    images: Vec<Capture>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Capture {
    name: String,
    sha256: String,
    snapshot: String,
    dom: String,
    native_metadata: String,
    error_logs: String,
    loaded_web_fonts: Vec<Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Artwork {
    license_file: String,
    license_sha256: String,
    glyphs: Vec<Glyph>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Glyph {
    file: String,
    svg_sha256: String,
    #[serde(default)]
    usage_overrides: Vec<Glyph>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentLogos {
    sources: Vec<LogoSource>,
    logos: Vec<Logo>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LogoSource {
    /// Absent for artwork supplied without a license file; `license` then states its terms.
    license_file: Option<String>,
    license_sha256: Option<String>,
    disclaimer_file: Option<String>,
    disclaimer_sha256: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Logo {
    agent: String,
    file: String,
    svg_sha256: String,
}

fn read(base: &Path, relative: &str) -> anyhow::Result<Vec<u8>> {
    ensure!(
        Path::new(relative)
            .components()
            .all(|p| matches!(p, Component::Normal(_))),
        "Evidence path must stay under its manifest directory: {relative}"
    );
    std::fs::read(base.join(relative)).with_context(|| format!("Missing evidence {relative}"))
}

fn verify_hash(base: &Path, relative: &str, expected: &str) -> anyhow::Result<()> {
    let actual = format!("{:x}", Sha256::digest(read(base, relative)?));
    ensure!(actual == expected, "Evidence changed: {relative}");
    Ok(())
}

pub fn verify(root: &Path) -> anyhow::Result<()> {
    let base = root.join("fixtures/baseline");
    let manifest: Baseline = serde_json::from_slice(&read(&base, "manifest.json")?)?;
    ensure!(
        manifest.baseline_revision == "83d55ef789a7cf256021f6620742e0e9021a8312",
        "Unexpected baseline revision"
    );
    ensure!(
        manifest.image_count == 88 && manifest.images.len() == 88,
        "Baseline capture inventory is incomplete"
    );
    for capture in manifest.images {
        verify_hash(&base, &capture.name, &capture.sha256)?;
        ensure!(
            capture.loaded_web_fonts.is_empty(),
            "Baseline contains development web fonts: {}",
            capture.name
        );
        for path in [
            &capture.snapshot,
            &capture.dom,
            &capture.native_metadata,
            &capture.error_logs,
        ] {
            serde_json::from_slice::<Value>(&read(&base, path)?)
                .with_context(|| format!("Invalid evidence JSON {path}"))?;
        }
    }
    let assets = root.join("assets/hugeicons");
    let artwork: Artwork = serde_json::from_slice(&read(&assets, "inventory.json")?)?;
    verify_hash(&assets, &artwork.license_file, &artwork.license_sha256)?;
    for glyph in artwork.glyphs {
        verify_hash(&assets, &glyph.file, &glyph.svg_sha256)?;
        for variant in glyph.usage_overrides {
            verify_hash(&assets, &variant.file, &variant.svg_sha256)?;
        }
    }
    verify_agent_logos(root)
}

/// Agent logo artwork keeps its source licenses and names only registry agents.
fn verify_agent_logos(root: &Path) -> anyhow::Result<()> {
    let assets = root.join("assets/agent-logos");
    let inventory: AgentLogos = serde_json::from_slice(&read(&assets, "inventory.json")?)?;
    for source in inventory.sources {
        if let (Some(file), Some(sha256)) = (source.license_file, source.license_sha256) {
            verify_hash(&assets, &file, &sha256)?;
        }
        if let (Some(file), Some(sha256)) = (source.disclaimer_file, source.disclaimer_sha256) {
            verify_hash(&assets, &file, &sha256)?;
        }
    }
    let registry: Value =
        serde_json::from_slice(&read(&root.join("fixtures/contracts"), "registry-79.json")?)?;
    let known = |id: &str| {
        registry["agents"]
            .as_array()
            .is_some_and(|agents| agents.iter().any(|agent| agent["id"] == id))
    };
    for logo in inventory.logos {
        ensure!(known(&logo.agent), "Logo for unknown agent {}", logo.agent);
        verify_hash(&assets, &logo.file, &logo.svg_sha256)?;
    }
    Ok(())
}
