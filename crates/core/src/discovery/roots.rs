use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanRoot {
    pub id: String,
    pub canonical_path: PathBuf,
    pub display_path: String,
    pub label: String,
    pub enabled: bool,
    pub created_at: u64,
}
