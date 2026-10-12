//! Root and projects: machine-local deployment targets, their agents, and recorded deployments.

mod operations;
pub(crate) mod progress;
mod record;
pub(crate) mod scan;
mod store;
#[cfg(test)]
mod tests;

use skillbinder_proto::{DeploymentTarget, ProjectId};

/// Storage key of a target; project identities are UUIDs and never equal `root`.
fn key(target: &DeploymentTarget) -> &str {
    match target {
        DeploymentTarget::Root => "root",
        DeploymentTarget::Project(id) => id.as_str(),
    }
}

fn target(key: String) -> DeploymentTarget {
    if key == "root" {
        DeploymentTarget::Root
    } else {
        DeploymentTarget::Project(ProjectId(key))
    }
}
