//! Global and workspaces: machine-local deployment targets, their agents, and recorded deployments.

mod deploy;
#[cfg(test)]
mod deploy_tests;
mod drift;
mod importables;
mod journal;
mod operations;
pub(crate) mod progress;
mod record;
pub(crate) mod scan;
mod store;
#[cfg(test)]
mod tests;

use skillbinder_proto::{DeploymentTarget, WorkspaceId};

/// Storage key of a target; workspace identities are UUIDs and never equal `global`.
fn key(target: &DeploymentTarget) -> &str {
    match target {
        DeploymentTarget::Global => "global",
        DeploymentTarget::Workspace(id) => id.as_str(),
    }
}

fn target(key: String) -> DeploymentTarget {
    if key == "global" {
        DeploymentTarget::Global
    } else {
        DeploymentTarget::Workspace(WorkspaceId(key))
    }
}
