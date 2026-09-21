use serde::{Deserialize, Serialize};
use ts_rs::TS;

contract! {
    pub struct DiagnosticsRevealLogsResponse {
        pub path: String,
    }
}
