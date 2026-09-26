use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use unicode_casefold::UnicodeCaseFold;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Folder {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Tag {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillAssignment {
    #[serde(skip)]
    pub display_name: Option<String>,
    pub folder_id: Option<String>,
    pub tag_ids: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Organization {
    pub folders: BTreeMap<String, Folder>,
    pub tags: BTreeMap<String, Tag>,
    pub skills: BTreeMap<String, SkillAssignment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Change {
    CreateFolder {
        id: String,
        name: String,
    },
    UpdateFolder {
        id: String,
        name: String,
    },
    DeleteFolder {
        id: String,
    },
    CreateTag {
        id: String,
        name: String,
    },
    RenameTag {
        id: String,
        name: String,
    },
    DeleteTag {
        id: String,
    },
    Assign {
        skill_ids: Vec<String>,
        folder_id: Option<String>,
        set_folder: bool,
        add_tag_ids: Vec<String>,
        remove_tag_ids: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteImpact {
    pub affected_skills: usize,
}

fn key(name: &str) -> String {
    name.nfkc().case_fold().collect::<String>()
}

fn clean(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().any(char::is_control) || name.chars().count() > 100 {
        return Err("Names must contain 1–100 visible characters.".into());
    }
    Ok(name.to_owned())
}

fn safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 100
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

impl Organization {
    pub fn validate(&self) -> Result<(), String> {
        let mut folders = BTreeSet::new();
        for (id, folder) in &self.folders {
            if !safe_id(id) || id != &folder.id || clean(&folder.name)? != folder.name {
                return Err("Invalid folder record.".into());
            }
            if !folders.insert(key(&folder.name)) {
                return Err("A folder with this name already exists.".into());
            }
        }
        let mut tags = BTreeSet::new();
        for (id, tag) in &self.tags {
            if !safe_id(id) || id != &tag.id || clean(&tag.name)? != tag.name {
                return Err("Invalid tag record.".into());
            }
            if !tags.insert(key(&tag.name)) {
                return Err("A tag with this name already exists.".into());
            }
        }
        for (skill_id, assignment) in &self.skills {
            if !safe_id(skill_id) {
                return Err("Invalid skill ID.".into());
            }
            if assignment
                .folder_id
                .as_ref()
                .is_some_and(|id| !self.folders.contains_key(id))
            {
                return Err("A skill refers to a missing folder.".into());
            }
            if assignment.tag_ids.windows(2).any(|pair| pair[0] >= pair[1])
                || assignment
                    .tag_ids
                    .iter()
                    .any(|id| !self.tags.contains_key(id))
            {
                return Err("A skill has invalid tags.".into());
            }
        }
        Ok(())
    }

    pub fn preview_delete(
        &self,
        folder_id: Option<&str>,
        id: &str,
    ) -> Result<DeleteImpact, String> {
        if let Some(id) = folder_id {
            let mut proposed = self.clone();
            if proposed.folders.remove(id).is_none() {
                return Err("Folder not found.".into());
            }
            for skill in proposed
                .skills
                .values_mut()
                .filter(|skill| skill.folder_id.as_deref() == Some(id))
            {
                skill.folder_id = None;
            }
            proposed.validate()?;
            Ok(DeleteImpact {
                affected_skills: self
                    .skills
                    .values()
                    .filter(|skill| skill.folder_id.as_deref() == Some(id))
                    .count(),
            })
        } else {
            if !self.tags.contains_key(id) {
                return Err("Tag not found.".into());
            }
            Ok(DeleteImpact {
                affected_skills: self
                    .skills
                    .values()
                    .filter(|skill| skill.tag_ids.iter().any(|tag| tag == id))
                    .count(),
            })
        }
    }

    pub fn apply(&self, change: Change) -> Result<Self, String> {
        let mut next = self.clone();
        match change {
            Change::CreateFolder { id, name } => {
                if next.folders.contains_key(&id) {
                    return Err("Folder ID already exists.".into());
                }
                next.folders.insert(
                    id.clone(),
                    Folder {
                        id,
                        name: clean(&name)?,
                    },
                );
            }
            Change::UpdateFolder { id, name } => {
                let folder = next.folders.get_mut(&id).ok_or("Folder not found.")?;
                folder.name = clean(&name)?;
            }
            Change::DeleteFolder { id } => {
                self.preview_delete(Some(&id), &id)?;
                if next.folders.remove(&id).is_none() {
                    return Err("Folder not found.".into());
                }
                for skill in next
                    .skills
                    .values_mut()
                    .filter(|skill| skill.folder_id.as_deref() == Some(&id))
                {
                    skill.folder_id = None;
                }
            }
            Change::CreateTag { id, name } => {
                if next.tags.contains_key(&id) {
                    return Err("Tag ID already exists.".into());
                }
                next.tags.insert(
                    id.clone(),
                    Tag {
                        id,
                        name: clean(&name)?,
                    },
                );
            }
            Change::RenameTag { id, name } => {
                next.tags.get_mut(&id).ok_or("Tag not found.")?.name = clean(&name)?
            }
            Change::DeleteTag { id } => {
                if next.tags.remove(&id).is_none() {
                    return Err("Tag not found.".into());
                }
                for skill in next.skills.values_mut() {
                    skill.tag_ids.retain(|tag| tag != &id);
                }
            }
            Change::Assign {
                skill_ids,
                folder_id,
                set_folder,
                add_tag_ids,
                remove_tag_ids,
            } => {
                if skill_ids.is_empty() {
                    return Err("Select at least one skill.".into());
                }
                for id in skill_ids {
                    let skill = next.skills.get_mut(&id).ok_or("Skill not found.")?;
                    if set_folder {
                        skill.folder_id = folder_id.clone();
                    }
                    let mut tags: BTreeSet<_> = skill.tag_ids.iter().cloned().collect();
                    tags.extend(add_tag_ids.iter().cloned());
                    for tag in &remove_tag_ids {
                        tags.remove(tag);
                    }
                    skill.tag_ids = tags.into_iter().collect();
                }
            }
        }
        next.validate()?;
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_rules_and_folder_delete() {
        let mut graph = Organization::default();
        graph
            .skills
            .insert("skill".into(), SkillAssignment::default());
        graph = graph
            .apply(Change::CreateFolder {
                id: "a".into(),
                name: "First".into(),
            })
            .unwrap();
        graph = graph
            .apply(Change::CreateFolder {
                id: "b".into(),
                name: "Second".into(),
            })
            .unwrap();
        assert!(
            graph
                .apply(Change::CreateFolder {
                    id: "c".into(),
                    name: "FIRST".into(),
                })
                .is_err()
        );
        graph = graph
            .apply(Change::CreateTag {
                id: "one".into(),
                name: "Straße".into(),
            })
            .unwrap();
        assert!(
            graph
                .apply(Change::CreateTag {
                    id: "two".into(),
                    name: "STRASSE".into()
                })
                .is_err()
        );
        assert!(
            graph
                .apply(Change::UpdateFolder {
                    id: "b".into(),
                    name: "First".into()
                })
                .is_err()
        );
        graph = graph
            .apply(Change::UpdateFolder {
                id: "b".into(),
                name: "Renamed".into(),
            })
            .unwrap();
        assert_eq!(graph.folders["b"].name, "Renamed");
        graph = graph
            .apply(Change::Assign {
                skill_ids: vec!["skill".into()],
                folder_id: Some("a".into()),
                set_folder: true,
                add_tag_ids: vec![],
                remove_tag_ids: vec![],
            })
            .unwrap();
        assert_eq!(
            graph.preview_delete(Some("a"), "a").unwrap(),
            DeleteImpact { affected_skills: 1 }
        );
        graph = graph
            .apply(Change::DeleteFolder { id: "a".into() })
            .unwrap();
        assert_eq!(graph.skills["skill"].folder_id, None);
        assert_eq!(graph.folders.len(), 1);
    }
}
