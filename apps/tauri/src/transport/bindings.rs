use serde::{Deserialize, Serialize};
use ts_rs::TS;

contract! {
    pub struct BindingAgent {
        pub agent_id: String,
        pub display_name: String,
        pub detected: bool,
        pub available: bool,
        pub reader_path: String,
    }
}
contract! {
    pub struct BindingsOptionsResponse {
        pub agents: Vec<BindingAgent>,
    }
}
contract! {
    pub struct BindingTargetView {
        pub skill_id: String,
        pub path: String,
        pub reader_agent_ids: Vec<String>,
        pub status: String,
    }
}
contract! {
    pub struct BindingView {
        pub binding_id: String,
        pub skill_ids: Vec<String>,
        pub scope: String,
        pub project_root_id: Option<String>,
        pub agent_ids: Vec<String>,
        pub created_at: String,
        pub targets: Vec<BindingTargetView>,
    }
}
contract! {
    pub struct BindingsListResponse {
        pub bindings: Vec<BindingView>,
    }
}
contract! {
    pub struct BindingsCreateResponse {
        pub binding: BindingView,
    }
}
contract! {
    pub struct BindingsRepairResponse {
        pub binding: BindingView,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BindingsOptionsRequest {
    pub project_root_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BindingsCreateRequest {
    pub skill_ids: Vec<String>,
    pub scope: String,
    pub project_root_id: Option<String>,
    pub agent_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BindingsRepairRequest {
    pub binding_id: String,
}
