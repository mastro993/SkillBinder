use serde::{Deserialize, Serialize};
use ts_rs::TS;

contract! {
    pub struct RootGrantView {
        pub grant_id: String,
        pub display_path: String,
        pub resolved_path: String,
    }
}

contract! {
    pub struct RootsPickResponse {
        pub grant: Option<RootGrantView>,
    }
}

contract! {
    pub struct RootView {
        pub root_id: String,
        pub display_path: String,
        pub resolved_path: String,
        pub label: String,
        pub enabled: bool,
    }
}

contract! {
    pub struct RootsListResponse {
        pub roots: Vec<RootView>,
    }
}

contract! {
    pub struct RootsRegisterResponse {
        pub root: RootView,
    }
}

contract! {
    pub struct RootsUpdateResponse {
        pub root: RootView,
    }
}

contract! {
    pub struct RootsRemoveResponse {
        pub root_id: String,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RootsRegisterRequest {
    pub grant_id: String,
    pub label: Option<String>,
}

impl<'de> Deserialize<'de> for RootsRegisterRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct WireRequest {
            grant_id: String,
            label: Option<String>,
        }

        let request = WireRequest::deserialize(deserializer)?;
        Ok(Self {
            grant_id: request.grant_id,
            label: request.label,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RootsUpdateRequest {
    pub root_id: String,
    pub label: String,
    pub enabled: bool,
}

impl<'de> Deserialize<'de> for RootsUpdateRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct WireRequest {
            root_id: String,
            label: String,
            enabled: bool,
        }

        let request = WireRequest::deserialize(deserializer)?;
        Ok(Self {
            root_id: request.root_id,
            label: request.label,
            enabled: request.enabled,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RootsRemoveRequest {
    pub root_id: String,
}

impl<'de> Deserialize<'de> for RootsRemoveRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct WireRequest {
            root_id: String,
        }

        let request = WireRequest::deserialize(deserializer)?;
        Ok(Self {
            root_id: request.root_id,
        })
    }
}
