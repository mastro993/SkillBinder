use serde::{Deserialize, Serialize};
use skillbinder_core::library::Manifest;
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortableMetadata {
    pub schema_version: u32,
    pub library_id: String,
    pub created_at: String,
    pub content_policy_version: u32,
    #[serde(default)]
    pub skills: BTreeMap<String, PortableSkill>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortableSkill {
    pub id: String,
    pub slug: String,
    pub display_name: Option<String>,
    pub folder_id: Option<String>,
    pub tag_ids: Vec<String>,
    pub upstream_bindings: Vec<serde_json::Value>,
    pub manifest: Manifest,
}

impl PortableMetadata {
    pub fn new(library_id: String, created_at: String) -> Self {
        Self {
            schema_version: 1,
            library_id,
            created_at,
            content_policy_version: 1,
            skills: BTreeMap::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|error| error.to_string())?;
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())
    }

    pub fn write(&self, path: &Path) -> Result<(), String> {
        let temporary = path.with_extension("json.tmp");
        let bytes = format!(
            "{}\n",
            serde_json::to_string_pretty(self).map_err(|error| error.to_string())?
        );
        fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
        fs::rename(temporary, path).map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use skillbinder_core::library::{Manifest, ManifestEntry, ManifestKind};

    #[test]
    fn round_trips_single_file_metadata() {
        let directory =
            std::env::temp_dir().join(format!("skillbinder-metadata-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join(".skillbinder.json");
        let mut metadata = PortableMetadata::new("library".into(), "created".into());
        metadata.skills.insert(
            "skill".into(),
            PortableSkill {
                id: "skill".into(),
                slug: "review".into(),
                display_name: None,
                folder_id: None,
                tag_ids: Vec::new(),
                upstream_bindings: Vec::new(),
                manifest: Manifest::new(vec![ManifestEntry {
                    kind: ManifestKind::File,
                    path: "SKILL.md".into(),
                    sha256: "00".repeat(32),
                    bytes: 1,
                    executable: false,
                }]),
            },
        );
        metadata.write(&path).unwrap();
        let restored = PortableMetadata::load(&path).unwrap();
        assert_eq!(restored.skills["skill"].slug, "review");
        assert!(!directory.join(".skillbinder").exists());
        let _ = fs::remove_dir_all(directory);
    }
}
