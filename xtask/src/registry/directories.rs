//! The reviewed overlay of every skill directory each agent reads, beside the registry's own.
//!
//! The overlay is split so each file stays small enough to read whole: a header tying it to the
//! reviewed registry, then one file per range of agent names. A path compares after expansion
//! with every variable unset, as the engine resolves it on a device where no variable is set.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path},
};

use anyhow::{Context, ensure};
use serde::{Deserialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

use super::{Agent, REGISTRY_SHA256, Registry};

/// The reviewed bytes of the overlay: its header first, then one file per range of agent names.
const REGISTRY_DIRECTORIES_SHA256: [(&str, &str); 4] = [
    (
        "fixtures/contracts/registry-directories.json",
        "11fc2f1cd73221bf335dc014cb7cfee7baa34d5f32cfcc47d92cea189f39c5a5",
    ),
    (
        "fixtures/contracts/registry-directories/a-c.json",
        "ed6bc31e62166f353b2057df4dec097560125ff4de38cec53e5607ac5d6e11cf",
    ),
    (
        "fixtures/contracts/registry-directories/d-g.json",
        "db737257dfdf9128d65d3eba81ee627931e935c7d6d99886d0370544bd9bba9a",
    ),
    (
        "fixtures/contracts/registry-directories/k-z.json",
        "cb99a9a3014bf6c7c6b50d35aebfb6579459100a12de21c481c51c231f163124",
    ),
];
const OVERLAY_VERSION: u32 = 1;
/// The shared directories the registry's universal entries name, plus the one its others do.
const SHARED: [&str; 3] = [
    ".agents/skills",
    "~/.agents/skills",
    "${XDG_CONFIG_HOME:-~/.config}/agents/skills",
];
/// Hosts whose sources pin a full commit SHA the URL also names.
const REPOSITORIES: [&str; 5] = [
    "bitbucket.org",
    "codeberg.org",
    "git.sr.ht",
    "github.com",
    "gitlab.com",
];

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Overlay {
    overlay_version: u32,
    registry_sha256: String,
    #[serde(default)]
    agents: Vec<Entry>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Entry {
    agent: String,
    /// The surface an entry describes where a product's surfaces order directories differently.
    #[serde(default)]
    surface: Option<String>,
    workspace: Vec<Directory>,
    global: Vec<Directory>,
    same_slug: SameSlug,
    /// Tiers of directories a source leaves unranked against each other, winner first.
    #[serde(default)]
    order: Option<Vec<Vec<String>>>,
    sources: Vec<Source>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Directory {
    path: String,
    role: Role,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum Role {
    /// A directory named for this agent.
    Own,
    /// One of the three shared directories.
    Shared,
    /// Another agent's directory, or a variant no registry agent uses.
    Compatibility,
    /// A registry directory no source confirms.
    Unconfirmed,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum SameSlug {
    /// One copy wins in an order a source gives.
    Precedence,
    /// One copy wins, and no source fixes which.
    One,
    /// The agent keeps every copy.
    Both,
    /// No source says what it does.
    Unknown,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Source {
    url: String,
    /// The full commit SHA of the source on a repository host.
    #[serde(default)]
    revision: Option<String>,
    /// The date a source published only on a website was retrieved.
    #[serde(default)]
    retrieved_at: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Workspace,
    Global,
}

/// Reads the overlay, rejecting bytes that differ from the reviewed ones.
pub(super) fn load(root: &Path) -> anyhow::Result<Overlay> {
    for (path, expected) in REGISTRY_DIRECTORIES_SHA256 {
        let bytes = std::fs::read(root.join(path))
            .with_context(|| format!("Overlay file is missing: {path}"))?;
        ensure!(
            format!("{:x}", Sha256::digest(&bytes)) == expected,
            "Reviewed overlay bytes changed: {path}; a new provenance review is required"
        );
    }
    let header: Overlay = read(root, REGISTRY_DIRECTORIES_SHA256[0].0)?;
    let parts: Vec<Vec<Entry>> = REGISTRY_DIRECTORIES_SHA256[1..]
        .iter()
        .map(|(path, _)| read(root, path))
        .collect::<anyhow::Result<_>>()?;
    Ok(Overlay {
        agents: parts.into_iter().flatten().collect(),
        ..header
    })
}

/// Every rule the reviewed overlay must pass to describe the reviewed registry.
pub(super) fn check(registry: &Registry, overlay: &Overlay) -> anyhow::Result<()> {
    ensure!(
        overlay.overlay_version == OVERLAY_VERSION,
        "Overlay version changed; a new review is required"
    );
    ensure!(
        overlay.registry_sha256 == REGISTRY_SHA256,
        "Overlay registrySha256 does not match the reviewed registry"
    );
    let agents: BTreeMap<&str, &Agent> = registry
        .agents
        .iter()
        .map(|agent| (agent.id.as_str(), agent))
        .collect();
    let mut surfaces = BTreeSet::new();
    for entry in &overlay.agents {
        let agent = agents.get(entry.agent.as_str()).with_context(|| {
            format!(
                "Overlay names an agent outside the registry: {}",
                entry.agent
            )
        })?;
        ensure!(
            surfaces.insert((entry.agent.as_str(), entry.surface.as_deref())),
            "Overlay repeats an agent and surface: {} {:?}",
            entry.agent,
            entry.surface
        );
        entry.check(agent)?;
    }
    Ok(())
}

/// The directories an agent reads that are not its registry ones, workspace before global and
/// merged across its surfaces.
pub(super) fn others(overlay: &Overlay, agent: &Agent) -> Vec<String> {
    let registry = registry_dirs(agent);
    let mut seen = BTreeSet::new();
    let mut paths = Vec::new();
    for entry in overlay
        .agents
        .iter()
        .filter(|entry| entry.agent == agent.id)
    {
        for directory in entry.workspace.iter().chain(entry.global.iter()) {
            let Some(path) = normalized(&directory.path) else {
                continue;
            };
            if !registry.contains(&path) && seen.insert(path) {
                paths.push(directory.path.clone());
            }
        }
    }
    paths
}

impl Entry {
    fn check(&self, agent: &Agent) -> anyhow::Result<()> {
        let registry = registry_dirs(agent);
        let workspace = directories(&self.workspace, Side::Workspace, &registry)?;
        let global = directories(&self.global, Side::Global, &registry)?;
        let own = normalized(&agent.workspace_skills_dir)
            .context("Registry workspace directory does not expand")?;
        ensure!(
            workspace.contains(&own),
            "Overlay omits the registry workspace directory of {}: {}",
            agent.id,
            agent.workspace_skills_dir
        );
        for root in &agent.global_roots {
            ensure!(
                global.contains(
                    &normalized(root).context("Registry global directory does not expand")?
                ),
                "Overlay omits a registry global directory of {}: {root}",
                agent.id
            );
        }
        self.check_order(&workspace, &global)?;
        ensure!(!self.sources.is_empty(), "Overlay entry has no source");
        self.sources.iter().try_for_each(Source::check)
    }

    /// `order` is required where a source ranks the directories, and absent where it does not.
    fn check_order(
        &self,
        workspace: &BTreeSet<String>,
        global: &BTreeSet<String>,
    ) -> anyhow::Result<()> {
        let Some(order) = &self.order else {
            ensure!(
                self.same_slug != SameSlug::Precedence,
                "Overlay needs an order for a same-slug precedence"
            );
            return Ok(());
        };
        ensure!(
            matches!(self.same_slug, SameSlug::Precedence | SameSlug::One),
            "Overlay gives an order for an undecided same-slug behavior: {:?}",
            self.same_slug
        );
        let listed: BTreeSet<String> = workspace.union(global).cloned().collect();
        let mut named = BTreeSet::new();
        for tier in order {
            ensure!(!tier.is_empty(), "Overlay has an empty order tier");
            for name in tier {
                let path = normalized(name)
                    .with_context(|| format!("Overlay order path does not expand: {name}"))?;
                ensure!(
                    listed.contains(&path),
                    "Overlay order names an unlisted path: {name}"
                );
                ensure!(
                    named.insert(path),
                    "Overlay order names a path twice: {name}"
                );
            }
        }
        if self.same_slug == SameSlug::Precedence {
            ensure!(
                named == listed,
                "Overlay order leaves listed paths unranked"
            );
        }
        Ok(())
    }
}

/// The entry's directories on one side, rejecting a path another one already names.
fn directories(
    dirs: &[Directory],
    side: Side,
    registry: &BTreeSet<String>,
) -> anyhow::Result<BTreeSet<String>> {
    let mut paths = BTreeSet::new();
    for directory in dirs {
        let path = directory.check(side, registry)?;
        ensure!(
            paths.insert(path),
            "Overlay lists a path twice after expansion: {}",
            directory.path
        );
    }
    Ok(paths)
}

impl Directory {
    fn check(&self, side: Side, registry: &BTreeSet<String>) -> anyhow::Result<String> {
        let raw = self.path.as_str();
        ensure!(!raw.trim().is_empty(), "Overlay lists an empty path");
        ensure!(
            !Path::new(raw)
                .components()
                .any(|component| matches!(component, Component::ParentDir)),
            "Overlay path leaves its directory: {raw}"
        );
        match side {
            Side::Workspace => ensure!(
                !raw.starts_with('/') && !raw.starts_with('~') && !raw.starts_with("${"),
                "Overlay workspace path is not relative: {raw}"
            ),
            Side::Global => ensure!(
                raw.starts_with('~') || raw.starts_with("${") || raw.starts_with('/'),
                "Overlay global path does not start with ~, ${{, or /: {raw}"
            ),
        }
        let path =
            normalized(raw).with_context(|| format!("Overlay path does not expand: {raw}"))?;
        ensure!(!path.is_empty(), "Overlay path resolves to nothing: {raw}");
        match self.role {
            Role::Shared => ensure!(
                SHARED
                    .iter()
                    .filter_map(|template| normalized(template))
                    .any(|shared| shared == path),
                "Overlay marks a path outside the shared directories as shared: {raw}"
            ),
            Role::Unconfirmed => ensure!(
                registry.contains(&path),
                "Overlay marks a directory no registry entry lists as unconfirmed: {raw}"
            ),
            Role::Own | Role::Compatibility => {}
        }
        Ok(path)
    }
}

impl Source {
    fn check(&self) -> anyhow::Result<()> {
        let host = host(&self.url)
            .with_context(|| format!("Overlay source is not an https URL: {}", self.url))?;
        if !REPOSITORIES.contains(&host.as_str()) {
            ensure!(
                self.revision.is_none(),
                "Overlay source pins a revision outside a repository: {}",
                self.url
            );
            let retrieved = self
                .retrieved_at
                .as_deref()
                .with_context(|| format!("Overlay source has no retrievedAt date: {}", self.url))?;
            ensure!(
                is_date(retrieved),
                "Overlay source retrievedAt is not a date: {}",
                self.url
            );
            return Ok(());
        }
        let revision = self
            .revision
            .as_deref()
            .with_context(|| format!("Overlay source has no revision: {}", self.url))?;
        ensure!(
            revision.len() == 40 && revision.chars().all(|c| c.is_ascii_hexdigit()),
            "Overlay source revision is not a full commit SHA: {}",
            self.url
        );
        ensure!(
            self.url.contains(revision),
            "Overlay source URL does not name its revision: {}",
            self.url
        );
        Ok(())
    }
}

fn read<T: DeserializeOwned>(root: &Path, path: &str) -> anyhow::Result<T> {
    let bytes = std::fs::read(root.join(path))?;
    serde_json::from_slice(&bytes).with_context(|| format!("Invalid overlay file: {path}"))
}

/// The registry's directories for one agent, as the overlay compares them.
fn registry_dirs(agent: &Agent) -> BTreeSet<String> {
    agent
        .global_roots
        .iter()
        .chain(std::iter::once(&agent.workspace_skills_dir))
        .filter_map(|template| normalized(template))
        .collect()
}

/// A template as the engine expands it with every variable unset: `${VAR:-default}` becomes
/// `default`, and a bare `${VAR}` fails, since the engine resolves it to nothing.
fn normalized(template: &str) -> Option<String> {
    let mut output = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("${") {
        output.push_str(&rest[..start]);
        let tail = &rest[start + 2..];
        let end = tail.find('}')?;
        let (_, fallback) = tail[..end].split_once(":-")?;
        output.push_str(fallback);
        rest = &tail[end + 1..];
    }
    output.push_str(rest);
    let absolute = output.starts_with('/');
    let mut parts = Vec::new();
    for part in output.split('/') {
        match part {
            "" | "." => {}
            ".." => return None,
            part => parts.push(part),
        }
    }
    let joined = parts.join("/");
    Some(if absolute {
        format!("/{joined}")
    } else {
        joined
    })
}

fn host(url: &str) -> Option<String> {
    let rest = url.strip_prefix("https://")?;
    let host = rest.split(['/', '?', '#']).next()?;
    (!host.is_empty()).then(|| host.to_owned())
}

fn is_date(text: &str) -> bool {
    let mut parts = text.split('-');
    let (Some(year), Some(month), Some(day), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    year.len() == 4
        && month.len() == 2
        && day.len() == 2
        && [year, month, day]
            .iter()
            .all(|part| part.chars().all(|c| c.is_ascii_digit()))
}

#[cfg(test)]
mod tests;
