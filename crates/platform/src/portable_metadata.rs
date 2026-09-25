use serde::{Deserialize, Serialize};
use serde_json::Value;
use skillbinder_core::library::{
    Manifest,
    organization::{Folder, Tag},
};
use std::{collections::BTreeMap, fs, path::Path};

pub const SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortableMetadata {
    pub schema_version: u32,
    pub library_id: String,
    pub created_at: String,
    pub content_policy_version: u32,
    #[serde(default)]
    pub folders: BTreeMap<String, Folder>,
    #[serde(default)]
    pub tags: BTreeMap<String, Tag>,
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
    pub upstream_bindings: Vec<Value>,
    pub digest: String,
    pub file_count: u32,
    pub total_bytes: u64,
}

impl PortableSkill {
    pub fn from_manifest(id: String, slug: String, manifest: &Manifest) -> Self {
        Self {
            id,
            slug,
            display_name: None,
            folder_id: None,
            tag_ids: Vec::new(),
            upstream_bindings: Vec::new(),
            digest: manifest.digest.clone(),
            file_count: manifest.file_count(),
            total_bytes: manifest.total_bytes(),
        }
    }
}

impl PortableMetadata {
    pub fn new(library_id: String, created_at: String) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            library_id,
            created_at,
            content_policy_version: 1,
            folders: BTreeMap::new(),
            tags: BTreeMap::new(),
            skills: BTreeMap::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let bytes = fs::read(path).map_err(|error| error.to_string())?;
        Self::parse(&bytes)
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let value: Value = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        let version = value
            .get("schemaVersion")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        if version != u64::from(SCHEMA_VERSION) {
            return Err(format!(
                "library metadata schema {version} is not supported; this app reads and writes schema {SCHEMA_VERSION}"
            ));
        }
        serde_json::from_value(value).map_err(|error| error.to_string())
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
    fn round_trips_skill_totals_without_the_file_ledger() {
        let directory = std::env::temp_dir().join(format!(
            "skillbinder-metadata-round-trip-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join(".skillbinder.json");
        let manifest = ledger_manifest();
        let mut metadata = PortableMetadata::new("library".into(), "created".into());
        metadata.skills.insert(
            "skill".into(),
            PortableSkill::from_manifest("skill".into(), "review".into(), &manifest),
        );
        metadata.write(&path).unwrap();

        let written = fs::read_to_string(&path).unwrap();
        assert!(!written.contains("entries"));
        assert!(!written.contains("manifest"));

        let restored = PortableMetadata::load(&path).unwrap();
        let skill = &restored.skills["skill"];
        assert_eq!(restored.schema_version, SCHEMA_VERSION);
        assert_eq!(skill.slug, "review");
        assert_eq!(skill.digest, manifest.digest);
        assert_eq!(skill.file_count, 2);
        assert_eq!(skill.total_bytes, 50);
        assert!(!directory.join(".skillbinder").exists());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn rejects_every_schema_this_build_does_not_write() {
        for version in [SCHEMA_VERSION - 1, SCHEMA_VERSION + 1] {
            let other = format!(
                "{{\"schemaVersion\": {version}, \"libraryId\": \"l\", \"createdAt\": \"c\", \"contentPolicyVersion\": 1}}"
            );
            assert!(PortableMetadata::parse(other.as_bytes()).is_err());
        }
    }

    fn ledger_manifest() -> Manifest {
        Manifest::new(vec![
            ManifestEntry {
                kind: ManifestKind::File,
                path: "SKILL.md".into(),
                sha256: "00".repeat(32),
                bytes: 40,
                executable: false,
            },
            ManifestEntry {
                kind: ManifestKind::Directory,
                path: "assets".into(),
                sha256: "0".repeat(64),
                bytes: 0,
                executable: false,
            },
            ManifestEntry {
                kind: ManifestKind::File,
                path: "assets/notes.md".into(),
                sha256: "11".repeat(32),
                bytes: 10,
                executable: false,
            },
        ])
    }
}
