use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use skillbinder_proto::{AppError, AppResult};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
/// Materialized entry type; links never appear in manifests.
pub enum ManifestKind {
    /// A regular file.
    File,
    /// An explicit directory, including empty directories.
    Directory,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
/// One portable, relative payload entry.
pub struct ManifestEntry {
    /// Materialized file or directory.
    pub kind: ManifestKind,
    /// UTF-8 relative path with slash separators.
    pub path: String,
    /// Lowercase raw SHA-256 hex for files; empty for directories.
    pub sha256: String,
    /// File length; zero for directories.
    pub bytes: u64,
    /// Whether any Unix executable bit is set.
    pub executable: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
/// Sorted schema-1 payload manifest including explicit directories.
pub struct Manifest {
    /// Encoding version, currently one.
    pub schema_version: u32,
    /// Domain-separated SHA-256 with the sha256 prefix.
    pub digest: String,
    /// Entries sorted by path and then kind.
    pub entries: Vec<ManifestEntry>,
}

impl Manifest {
    pub(crate) fn from_entries(mut entries: Vec<ManifestEntry>) -> AppResult<Self> {
        entries.sort_by(|a, b| {
            a.path
                .cmp(&b.path)
                .then_with(|| kind_byte(a.kind).cmp(&kind_byte(b.kind)))
        });
        let mut hash = Sha256::new();
        hash.update(b"skillbinder-manifest-v1\n");
        for entry in &entries {
            hash.update([kind_byte(entry.kind)]);
            hash.update((entry.path.len() as u32).to_le_bytes());
            hash.update(entry.path.as_bytes());
            let mut content_hash = [0u8; 32];
            if entry.kind == ManifestKind::File {
                if entry.sha256.len() != 64 || !entry.sha256.is_ascii() {
                    return Err(AppError::validation("Invalid payload content digest."));
                }
                for (index, byte) in content_hash.iter_mut().enumerate() {
                    *byte = u8::from_str_radix(&entry.sha256[index * 2..index * 2 + 2], 16)
                        .map_err(|_| AppError::validation("Invalid payload content digest."))?;
                }
            }
            hash.update(content_hash);
            hash.update(entry.bytes.to_le_bytes());
            hash.update([u8::from(entry.executable)]);
        }
        Ok(Self {
            schema_version: 1,
            digest: format!("sha256:{:x}", hash.finalize()),
            entries,
        })
    }
    /// Counts materialized regular files.
    pub fn file_count(&self) -> u64 {
        self.entries
            .iter()
            .filter(|e| e.kind == ManifestKind::File)
            .count() as u64
    }
    /// Sums file bytes, excluding directory metadata.
    pub fn total_bytes(&self) -> u64 {
        self.entries.iter().map(|e| e.bytes).sum()
    }
    /// Compares digest and complete manifest contents.
    pub fn equivalent(&self, other: &Self) -> bool {
        self == other
    }
}
fn kind_byte(kind: ManifestKind) -> u8 {
    match kind {
        ManifestKind::File => 1,
        ManifestKind::Directory => 2,
    }
}
