use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ManifestKind {
    File,
    Directory,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub kind: ManifestKind,
    pub path: String,
    pub sha256: String,
    pub bytes: u64,
    pub executable: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub digest: String,
    pub entries: Vec<ManifestEntry>,
}
impl Manifest {
    pub fn new(mut entries: Vec<ManifestEntry>) -> Self {
        entries.sort_by(|a, b| {
            a.path
                .cmp(&b.path)
                .then_with(|| kind_order(a.kind).cmp(&kind_order(b.kind)))
        });
        let digest = digest_entries(&entries);
        Self {
            schema_version: 1,
            digest,
            entries,
        }
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn equivalent(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
    pub fn file_count(&self) -> u32 {
        self.entries
            .iter()
            .filter(|entry| entry.kind == ManifestKind::File)
            .count() as u32
    }
    pub fn total_bytes(&self) -> u64 {
        self.entries.iter().map(|entry| entry.bytes).sum()
    }
}
fn kind_order(kind: ManifestKind) -> u8 {
    match kind {
        ManifestKind::File => 1,
        ManifestKind::Directory => 2,
    }
}
pub fn digest_entries(entries: &[ManifestEntry]) -> String {
    let mut data = b"skillbinder-manifest-v1\n".to_vec();
    let mut sorted = entries.to_vec();
    sorted.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then_with(|| kind_order(a.kind).cmp(&kind_order(b.kind)))
    });
    for entry in sorted {
        data.push(kind_order(entry.kind));
        data.extend_from_slice(&(entry.path.len() as u32).to_le_bytes());
        data.extend_from_slice(entry.path.as_bytes());
        if entry.kind == ManifestKind::File {
            let bytes = hex_decode(&entry.sha256).unwrap_or([0; 32]);
            data.extend_from_slice(&bytes);
        } else {
            data.extend_from_slice(&[0; 32]);
        }
        data.extend_from_slice(&entry.bytes.to_le_bytes());
        data.push(u8::from(entry.executable));
    }
    let mut hash = Sha256::new();
    hash.update(data);
    format!("sha256:{:x}", hash.finalize())
}
fn hex_decode(value: &str) -> Result<[u8; 32], ()> {
    if value.len() != 64 {
        return Err(());
    }
    let mut out = [0; 32];
    for (i, b) in value.as_bytes().chunks_exact(2).enumerate() {
        out[i] = u8::from_str_radix(std::str::from_utf8(b).map_err(|_| ())?, 16).map_err(|_| ())?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn digest_deterministic() {
        let a = Manifest::new(vec![ManifestEntry {
            kind: ManifestKind::File,
            path: "x".into(),
            sha256: "00".repeat(32),
            bytes: 1,
            executable: false,
        }]);
        let b = Manifest::new(a.entries.clone());
        assert_eq!(a, b);
    }
}
