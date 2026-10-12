//! Global and projects: machine-local deployment targets, their agents, and recorded deployments.

mod operations;
pub(crate) mod progress;
mod record;
pub(crate) mod scan;
mod store;
#[cfg(test)]
mod tests;

use skillbinder_proto::{DeploymentTarget, ProjectId};

/// Storage key of a target; project identities are UUIDs and never equal `global`.
fn key(target: &DeploymentTarget) -> &str {
    match target {
        DeploymentTarget::Global => "global",
        DeploymentTarget::Project(id) => id.as_str(),
    }
}

fn target(key: String) -> DeploymentTarget {
    if key == "global" {
        DeploymentTarget::Global
    } else {
        DeploymentTarget::Project(ProjectId(key))
    }
}
