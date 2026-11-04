//! The reviewed overlay of every skill directory each agent reads, beside the registry's own.
//!
//! `xtask verify` holds the overlay to the registry it was reviewed against; the engine only
//! reads it, splitting it as it is stored: one file per range of agent names.

use serde::Deserialize;
use skillbinder_proto::{DirRole, SameSlug};

/// The overlay's parts, in the order `xtask` verifies them.
const PARTS: [&str; 3] = [
    include_str!("../../../../../fixtures/contracts/registry-directories/a-c.json"),
    include_str!("../../../../../fixtures/contracts/registry-directories/d-g.json"),
    include_str!("../../../../../fixtures/contracts/registry-directories/k-z.json"),
];

/// One part of the overlay; its agent range is verified by `xtask`, not read here.
#[derive(Deserialize)]
struct Part {
    agents: Vec<Entry>,
}

/// The directories one agent, or one surface of it, reads.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Entry {
    pub agent: String,
    pub workspace: Vec<Directory>,
    pub global: Vec<Directory>,
    pub same_slug: SameSlug,
}

#[derive(Deserialize)]
pub(super) struct Directory {
    pub path: String,
    pub role: DirRole,
}

/// Every overlay entry, in reviewed order: an agent with two surfaces appears twice.
pub(super) fn entries() -> Result<Vec<Entry>, serde_json::Error> {
    PARTS.iter().try_fold(Vec::new(), |mut all, part| {
        let part: Part = serde_json::from_str(part)?;
        all.extend(part.agents);
        Ok(all)
    })
}
