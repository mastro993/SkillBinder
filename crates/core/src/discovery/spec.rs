//! What a scan covers: the input, its containment, and the policy that bounds the walk.

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRoot {
    pub path: PathBuf,
    pub agent_id: String,
    pub agent_label: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanLimits {
    pub category_depth: usize,
    pub max_entries: usize,
    pub max_link_hops: u8,
}
impl Default for ScanLimits {
    fn default() -> Self {
        Self {
            category_depth: 8,
            max_entries: 5000,
            max_link_hops: 16,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanInput {
    pub root_id: Option<String>,
    pub path: PathBuf,
    pub agent_ids: Vec<String>,
    pub agent_labels: Vec<String>,
    pub containment: Containment,
    pub policy: ScanPolicy,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Containment {
    Home,
    Grant { canonical: PathBuf },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExclusionReason {
    VcsMetadata,
    DependencyVendor,
    BuildOutput,
    Cache,
    VirtualEnvironment,
    AppData,
    MountBoundary,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExclusionRule {
    pub name: &'static str,
    pub reason: ExclusionReason,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanExclusion {
    pub name: String,
    pub reason: ExclusionReason,
    pub matches: u32,
    pub sample_path: PathBuf,
}

const GLOBAL_EXCLUSIONS: &[ExclusionRule] = &[
    ExclusionRule {
        name: ".git",
        reason: ExclusionReason::VcsMetadata,
    },
    ExclusionRule {
        name: "node_modules",
        reason: ExclusionReason::DependencyVendor,
    },
];
const PROJECT_EXCLUSIONS: &[ExclusionRule] = &[
    ExclusionRule {
        name: ".git",
        reason: ExclusionReason::VcsMetadata,
    },
    ExclusionRule {
        name: ".hg",
        reason: ExclusionReason::VcsMetadata,
    },
    ExclusionRule {
        name: ".svn",
        reason: ExclusionReason::VcsMetadata,
    },
    ExclusionRule {
        name: "node_modules",
        reason: ExclusionReason::DependencyVendor,
    },
    ExclusionRule {
        name: "vendor",
        reason: ExclusionReason::DependencyVendor,
    },
    ExclusionRule {
        name: "Pods",
        reason: ExclusionReason::DependencyVendor,
    },
    ExclusionRule {
        name: "bower_components",
        reason: ExclusionReason::DependencyVendor,
    },
    ExclusionRule {
        name: "target",
        reason: ExclusionReason::BuildOutput,
    },
    ExclusionRule {
        name: "dist",
        reason: ExclusionReason::BuildOutput,
    },
    ExclusionRule {
        name: "build",
        reason: ExclusionReason::BuildOutput,
    },
    ExclusionRule {
        name: "out",
        reason: ExclusionReason::BuildOutput,
    },
    ExclusionRule {
        name: ".next",
        reason: ExclusionReason::BuildOutput,
    },
    ExclusionRule {
        name: ".nuxt",
        reason: ExclusionReason::BuildOutput,
    },
    ExclusionRule {
        name: ".svelte-kit",
        reason: ExclusionReason::BuildOutput,
    },
    ExclusionRule {
        name: "DerivedData",
        reason: ExclusionReason::BuildOutput,
    },
    ExclusionRule {
        name: ".cache",
        reason: ExclusionReason::Cache,
    },
    ExclusionRule {
        name: ".turbo",
        reason: ExclusionReason::Cache,
    },
    ExclusionRule {
        name: ".parcel-cache",
        reason: ExclusionReason::Cache,
    },
    ExclusionRule {
        name: ".gradle",
        reason: ExclusionReason::Cache,
    },
    ExclusionRule {
        name: "__pycache__",
        reason: ExclusionReason::Cache,
    },
    ExclusionRule {
        name: ".pytest_cache",
        reason: ExclusionReason::Cache,
    },
    ExclusionRule {
        name: ".mypy_cache",
        reason: ExclusionReason::Cache,
    },
    ExclusionRule {
        name: ".ruff_cache",
        reason: ExclusionReason::Cache,
    },
    ExclusionRule {
        name: ".venv",
        reason: ExclusionReason::VirtualEnvironment,
    },
    ExclusionRule {
        name: "venv",
        reason: ExclusionReason::VirtualEnvironment,
    },
    ExclusionRule {
        name: ".tox",
        reason: ExclusionReason::VirtualEnvironment,
    },
    ExclusionRule {
        name: "AppData",
        reason: ExclusionReason::AppData,
    },
    ExclusionRule {
        name: "Application Data",
        reason: ExclusionReason::AppData,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanPolicy {
    pub(super) limits: ScanLimits,
    pub(super) exclusions: &'static [ExclusionRule],
    pub(super) stop_at_mount: bool,
}
impl ScanPolicy {
    pub fn global(limits: ScanLimits) -> Self {
        Self {
            limits,
            exclusions: GLOBAL_EXCLUSIONS,
            stop_at_mount: false,
        }
    }
    pub fn project() -> Self {
        Self::project_with_limits(ScanLimits {
            category_depth: 12,
            max_entries: 200_000,
            max_link_hops: 16,
        })
    }
    pub fn project_with_limits(limits: ScanLimits) -> Self {
        Self {
            limits,
            exclusions: PROJECT_EXCLUSIONS,
            stop_at_mount: true,
        }
    }
}
