use crate::portable_metadata::{PortableMetadata, PortableSkill};
use chrono::{DateTime, SecondsFormat, Utc};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub const SYNC_COMMIT_PREFIX: &str = "chore(skills): SkillBinder sync";

/// Names one sync commit across every client. The UTC timestamp keeps the id readable in history
/// without depending on any local counter, and the random suffix separates two clients that commit
/// inside the same second.
pub fn new_sync_id(now: DateTime<Utc>) -> String {
    let suffix = Uuid::new_v4().simple().to_string();
    format!(
        "{}-{}",
        now.to_rfc3339_opts(SecondsFormat::Secs, true),
        &suffix[..4]
    )
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct CatalogChange {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub updated: Vec<String>,
}

impl CatalogChange {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.updated.is_empty()
    }
}

/// The portable catalog is the whole diffable description of library content, so comparing it
/// across a sync names every skill that entered, left, or moved.
pub fn catalog_change(
    before: Option<&PortableMetadata>,
    after: Option<&PortableMetadata>,
) -> CatalogChange {
    let empty = BTreeMap::new();
    let before = before.map(|metadata| &metadata.skills).unwrap_or(&empty);
    let after = after.map(|metadata| &metadata.skills).unwrap_or(&empty);
    let mut change = CatalogChange::default();
    for (id, previous) in before {
        match after.get(id) {
            Some(current) => {
                let fields = changed_fields(previous, current);
                if !fields.is_empty() {
                    change
                        .updated
                        .push(format!("{}: {}", current.slug, fields.join(", ")));
                }
            }
            None => change.removed.push(previous.slug.clone()),
        }
    }
    for (id, current) in after {
        if !before.contains_key(id) {
            change.added.push(current.slug.clone());
        }
    }
    change.added.sort();
    change.removed.sort();
    change.updated.sort();
    change
}

pub fn commit_message(id: &str, change: &CatalogChange, managed_files: Option<&str>) -> String {
    let mut body = String::new();
    section(&mut body, "Skills added", &change.added);
    section(&mut body, "Skills removed", &change.removed);
    section(&mut body, "Skills updated", &change.updated);
    if body.is_empty() {
        match managed_files
            .map(str::trim)
            .filter(|stats| !stats.is_empty())
        {
            Some(stats) => {
                body.push_str("Managed files changed: ");
                body.push_str(stats);
                body.push('\n');
            }
            None => body.push_str("Managed files updated.\n"),
        }
    }
    format!(
        "{SYNC_COMMIT_PREFIX} {id}\n\n{}\n\nOperation-ID: {id}\n",
        body.trim_end()
    )
}

fn section(body: &mut String, title: &str, lines: &[String]) {
    if lines.is_empty() {
        return;
    }
    if !body.is_empty() {
        body.push('\n');
    }
    body.push_str(title);
    body.push_str(":\n");
    for line in lines {
        body.push_str("- ");
        body.push_str(line);
        body.push('\n');
    }
}

fn changed_fields(before: &PortableSkill, after: &PortableSkill) -> Vec<String> {
    let mut fields = Vec::new();
    if before.slug != after.slug {
        fields.push(format!("slug {} -> {}", before.slug, after.slug));
    }
    if before.display_name != after.display_name {
        fields.push(format!(
            "name {} -> {}",
            label(before.display_name.as_deref()),
            label(after.display_name.as_deref())
        ));
    }
    if before.folder_id != after.folder_id {
        fields.push(format!(
            "folder {} -> {}",
            label(before.folder_id.as_deref()),
            label(after.folder_id.as_deref())
        ));
    }
    if let Some(tags) = tag_change(&before.tag_ids, &after.tag_ids) {
        fields.push(format!("tags {tags}"));
    }
    if before.digest != after.digest {
        fields.push(format!(
            "digest {} -> {}",
            short(&before.digest),
            short(&after.digest)
        ));
    }
    if before.file_count != after.file_count {
        fields.push(format!(
            "files {} -> {}",
            before.file_count, after.file_count
        ));
    }
    if before.total_bytes != after.total_bytes {
        fields.push(format!(
            "bytes {} -> {}",
            before.total_bytes, after.total_bytes
        ));
    }
    if before.upstream_bindings != after.upstream_bindings {
        fields.push("provenance updated".to_owned());
    }
    fields
}

fn tag_change(before: &[String], after: &[String]) -> Option<String> {
    let before: BTreeSet<&str> = before.iter().map(String::as_str).collect();
    let after: BTreeSet<&str> = after.iter().map(String::as_str).collect();
    let added: Vec<&str> = after.difference(&before).copied().collect();
    let removed: Vec<&str> = before.difference(&after).copied().collect();
    if added.is_empty() && removed.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    if !added.is_empty() {
        parts.push(format!("+{}", added.join(" +")));
    }
    if !removed.is_empty() {
        parts.push(format!("-{}", removed.join(" -")));
    }
    Some(parts.join(" "))
}

fn label(value: Option<&str>) -> &str {
    value.unwrap_or("none")
}

fn short(digest: &str) -> &str {
    &digest[..digest.len().min(12)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_ids_carry_a_utc_timestamp_and_stay_unique_within_a_second() {
        let now = DateTime::parse_from_rfc3339("2026-09-26T14:30:12Z")
            .unwrap()
            .to_utc();
        let first = new_sync_id(now);
        let second = new_sync_id(now);
        assert!(first.starts_with("2026-09-26T14:30:12Z-"), "{first}");
        assert_eq!(first.len(), "2026-09-26T14:30:12Z-".len() + 4);
        assert_ne!(first, second);
    }

    #[test]
    fn message_lists_added_removed_and_updated_skills() {
        let mut before = PortableMetadata::new("library".into(), "created".into());
        before.skills.insert(
            "gone".into(),
            skill("gone", "legacy", "1".repeat(64), vec!["t_1".into()]),
        );
        before.skills.insert(
            "kept".into(),
            skill("kept", "caveman", "2".repeat(64), vec!["t_1".into()]),
        );
        let mut after = PortableMetadata::new("library".into(), "created".into());
        let changed = PortableSkill {
            folder_id: Some("f_2".into()),
            tag_ids: vec!["t_2".into()],
            digest: "3".repeat(64),
            file_count: 9,
            ..skill("kept", "caveman", "2".repeat(64), vec!["t_1".into()])
        };
        after.skills.insert("kept".into(), changed);
        after.skills.insert(
            "new".into(),
            skill("new", "review", "4".repeat(64), Vec::new()),
        );

        let change = catalog_change(Some(&before), Some(&after));
        assert_eq!(change.added, ["review"]);
        assert_eq!(change.removed, ["legacy"]);
        assert_eq!(
            change.updated,
            [
                "caveman: folder none -> f_2, tags +t_2 -t_1, digest 222222222222 -> 333333333333, files 4 -> 9"
            ]
        );

        let message = commit_message("2026-09-26T14:30:12Z-3f9a", &change, None);
        assert_eq!(
            message,
            "chore(skills): SkillBinder sync 2026-09-26T14:30:12Z-3f9a\n\n\
             Skills added:\n- review\n\n\
             Skills removed:\n- legacy\n\n\
             Skills updated:\n\
             - caveman: folder none -> f_2, tags +t_2 -t_1, digest 222222222222 -> 333333333333, files 4 -> 9\n\
             \n\
             Operation-ID: 2026-09-26T14:30:12Z-3f9a\n"
        );
    }

    #[test]
    fn message_falls_back_to_the_managed_file_totals() {
        let metadata = PortableMetadata::new("library".into(), "created".into());
        let change = catalog_change(Some(&metadata), Some(&metadata));
        assert!(change.is_empty());
        let message = commit_message("id", &change, Some(" 1 file changed, 2 insertions(+)"));
        assert!(message.contains("Managed files changed: 1 file changed, 2 insertions(+)"));
        assert!(message.ends_with("Operation-ID: id\n"));
        let bare = commit_message("id", &change, None);
        assert!(bare.contains("Managed files updated."));
    }

    fn skill(id: &str, slug: &str, digest: String, tag_ids: Vec<String>) -> PortableSkill {
        PortableSkill {
            id: id.into(),
            slug: slug.into(),
            display_name: None,
            folder_id: None,
            tag_ids,
            upstream_bindings: Vec::new(),
            digest,
            file_count: 4,
            total_bytes: 100,
        }
    }
}
