use crate::transport::validation::{CandidateDuplicate, ValidationSummary};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

contract! {
    pub enum LocationState {
        Scanned,
        Missing,
        Unreadable,
    }
}

contract! {
    pub enum ExclusionReason {
        VcsMetadata,
        DependencyVendor,
        BuildOutput,
        Cache,
        VirtualEnvironment,
        AppData,
        MountBoundary,
    }
}

contract! {
    pub struct DiscoveryExclusion {
        pub name: String,
        pub reason: ExclusionReason,
        pub matches: u32,
        pub sample_path: String,
    }
}

contract! {
    pub struct DiscoveryWarning {
        pub display_path: Option<String>,
        pub message: String,
    }
}

contract! {
    pub struct DiscoveryLocation {
        pub location_id: String,
        pub root_id: Option<String>,
        pub display_path: String,
        pub agent_ids: Vec<String>,
        pub agent_labels: Vec<String>,
        pub state: LocationState,
        pub detail: Option<String>,
        pub limit_reached: bool,
    }
}

contract! {
    pub enum ScanPhase {
        Running,
        Finished,
        Cancelled,
        Failed,
    }
}

contract! {
    pub struct DiscoveryProgress {
        pub roots_total: u32,
        pub roots_done: u32,
        pub entries_seen: u32,
        pub candidates_found: u32,
        pub current_path: Option<String>,
    }
}

contract! {
    pub struct DiscoveryStartResponse {
        pub scan_id: String,
    }
}

contract! {
    pub struct DiscoveryCancelResponse {
        pub scan_id: String,
        pub accepted: bool,
    }
}

contract! {
    pub struct DiscoveryCurrentResponse {
        pub scan_id: Option<String>,
    }
}

contract! {
    pub struct DiscoveryCandidate {
        pub candidate_id: String,
        pub location_id: String,
        pub display_path: String,
        pub slug: String,
        pub name: Option<String>,
        pub description: Option<String>,
        pub reader_agent_ids: Vec<String>,
        pub validation: ValidationSummary,
        pub duplicate: CandidateDuplicate,
        pub file_count: u32,
        pub total_bytes: String,
        pub linked: bool,
        pub warnings: Vec<String>,
    }
}

contract! {
    pub struct DiscoveryResultsResponse {
        pub scan_id: String,
        pub phase: ScanPhase,
        pub registry_version: u32,
        pub progress: DiscoveryProgress,
        pub limits_reached: bool,
        pub locations: Vec<DiscoveryLocation>,
        pub exclusions: Vec<DiscoveryExclusion>,
        pub warnings: Vec<DiscoveryWarning>,
        pub candidates: Vec<DiscoveryCandidate>,
        pub total_candidates: u32,
        pub hidden_duplicates: u32,
        pub offset: u32,
        pub limit: u32,
        pub failure: Option<String>,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryResultsRequest {
    pub scan_id: String,
    pub offset: u32,
    pub limit: u32,
}

impl<'de> Deserialize<'de> for DiscoveryResultsRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct WireRequest {
            scan_id: String,
            offset: u32,
            limit: u32,
        }

        let request = WireRequest::deserialize(deserializer)?;
        Ok(Self {
            scan_id: request.scan_id,
            offset: request.offset,
            limit: request.limit,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryCancelRequest {
    pub scan_id: String,
}

impl<'de> Deserialize<'de> for DiscoveryCancelRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct WireRequest {
            scan_id: String,
        }

        let request = WireRequest::deserialize(deserializer)?;
        Ok(Self {
            scan_id: request.scan_id,
        })
    }
}
