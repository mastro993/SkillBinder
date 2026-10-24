//! Library rows in machine-local SQLite: skills with their cached payload facts, presets and
//! their memberships, and library folders.

use crate::{
    payload::Inspection,
    records::Record,
    schema::{library_folders, preset_skills, presets, skills},
};
use diesel::{prelude::*, upsert::excluded};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use skillbinder_proto::*;
use std::collections::{BTreeMap, BTreeSet};

/// The inspected payload facts of a skill directory and its record, as imports, updates, and
/// reconciliation save them.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct SavedSkill {
    pub slug: String,
    pub digest: String,
    pub file_count: u64,
    pub total_bytes: u64,
    pub details: SkillDetails,
    pub imported_at: Option<String>,
    #[serde(default)]
    pub source: Option<SkillSource>,
    /// The Git tree id the skill's record holds.
    #[serde(default)]
    pub recorded: String,
    #[serde(default)]
    pub drift: Option<Drift>,
}
impl SavedSkill {
    /// A skill whose content is `inspection` and whose record is `record`.
    pub(crate) fn new(
        slug: String,
        inspection: &Inspection,
        imported_at: Option<String>,
        record: Record,
        drift: Option<Drift>,
    ) -> Self {
        Self {
            slug,
            digest: inspection.manifest.digest.clone(),
            file_count: inspection.manifest.file_count(),
            total_bytes: inspection.manifest.total_bytes(),
            details: SkillDetails {
                description: inspection.description.clone(),
                validation: inspection.status,
                messages: inspection.messages.clone(),
            },
            imported_at,
            source: record.source,
            recorded: record.tree,
            drift,
        }
    }
}

/// Inserts a skill, or refreshes the payload facts and record of an existing one while keeping its
/// import time, folder, and presets.
pub(crate) fn save_skill(connection: &mut SqliteConnection, record: &SavedSkill) -> AppResult<()> {
    diesel::insert_into(skills::table)
        .values((
            skills::slug.eq(&record.slug),
            skills::digest.eq(&record.digest),
            skills::file_count.eq(record.file_count as i64),
            skills::total_bytes.eq(record.total_bytes as i64),
            skills::details.eq(to_json(&record.details)?),
            skills::imported_at.eq(&record.imported_at),
            skills::source.eq(record.source.as_ref().map(to_json).transpose()?),
            skills::recorded_tree.eq(&record.recorded),
            skills::drift.eq(record.drift.as_ref().map(to_text).transpose()?),
        ))
        .on_conflict(skills::slug)
        .do_update()
        .set((
            skills::digest.eq(excluded(skills::digest)),
            skills::file_count.eq(excluded(skills::file_count)),
            skills::total_bytes.eq(excluded(skills::total_bytes)),
            skills::details.eq(excluded(skills::details)),
            skills::source.eq(excluded(skills::source)),
            skills::recorded_tree.eq(excluded(skills::recorded_tree)),
            skills::drift.eq(excluded(skills::drift)),
        ))
        .execute(connection)
        .map(drop)
        .map_err(|_| AppError::storage())
}

/// Records the source of skill `slug`.
pub(crate) fn set_source(
    connection: &mut SqliteConnection,
    slug: &str,
    source: &SkillSource,
) -> AppResult<()> {
    diesel::update(skills::table.find(slug))
        .set(skills::source.eq(to_json(source)?))
        .execute(connection)
        .map(drop)
        .map_err(|_| AppError::storage())
}

/// The source the database caches for skill `slug`, if any.
pub(crate) fn cached_source(
    connection: &mut SqliteConnection,
    slug: &str,
) -> AppResult<Option<SkillSource>> {
    skills::table
        .find(slug)
        .select(skills::source)
        .first::<Option<String>>(connection)
        .optional()
        .map_err(|_| AppError::storage())?
        .flatten()
        .as_deref()
        .map(from_json)
        .transpose()
}

/// Records whether skill `slug` drifted from its record.
pub(crate) fn set_drift(
    connection: &mut SqliteConnection,
    slug: &str,
    drift: Option<Drift>,
) -> AppResult<()> {
    diesel::update(skills::table.find(slug))
        .set(skills::drift.eq(drift.as_ref().map(to_text).transpose()?))
        .execute(connection)
        .map(drop)
        .map_err(|_| AppError::storage())
}

/// Removes a skill; its preset memberships cascade away with it.
pub(crate) fn delete_skill(connection: &mut SqliteConnection, slug: &str) -> AppResult<()> {
    diesel::delete(skills::table.find(slug))
        .execute(connection)
        .map(drop)
        .map_err(|_| AppError::storage())
}

/// Whether the library holds `slug`.
pub(crate) fn holds(connection: &mut SqliteConnection, slug: &str) -> AppResult<bool> {
    skills::table
        .find(slug)
        .count()
        .get_result::<i64>(connection)
        .map(|rows| rows > 0)
        .map_err(|_| AppError::storage())
}

/// The slugs of the library skills whose record is not cached yet, such as skills imported
/// before records existed.
pub(crate) fn unrecorded(connection: &mut SqliteConnection) -> AppResult<BTreeSet<String>> {
    skills::table
        .filter(skills::recorded_tree.eq(""))
        .select(skills::slug)
        .load::<String>(connection)
        .map(BTreeSet::from_iter)
        .map_err(|_| AppError::storage())
}

/// The slugs of every library skill.
pub(crate) fn slugs(connection: &mut SqliteConnection) -> AppResult<BTreeSet<String>> {
    skills::table
        .select(skills::slug)
        .load::<String>(connection)
        .map(BTreeSet::from_iter)
        .map_err(|_| AppError::storage())
}

type SkillRow = (
    String,
    String,
    i64,
    i64,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
);

/// The library and each skill's cached details.
pub(crate) fn load(
    connection: &mut SqliteConnection,
) -> AppResult<(Library, BTreeMap<SkillId, SkillDetails>)> {
    let storage = |_| AppError::storage();
    let mut memberships = BTreeMap::<String, Vec<PresetId>>::new();
    for (preset, slug) in preset_skills::table
        .order(preset_skills::preset_id)
        .load::<(String, String)>(connection)
        .map_err(storage)?
    {
        memberships.entry(slug).or_default().push(PresetId(preset));
    }
    let mut details = BTreeMap::new();
    let mut skills = Vec::new();
    for (
        slug,
        digest,
        file_count,
        total_bytes,
        detail,
        imported_at,
        folder,
        source,
        recorded,
        drift,
    ) in skills::table
        .order(skills::slug)
        .load::<SkillRow>(connection)
        .map_err(storage)?
    {
        let id = SkillId(slug);
        details.insert(id.clone(), from_json(&detail)?);
        skills.push(Skill {
            preset_ids: memberships.remove(id.as_str()).unwrap_or_default(),
            id,
            folder_id: folder.map(FolderId),
            digest,
            file_count: file_count as u64,
            total_bytes: total_bytes as u64,
            imported_at,
            source: source.as_deref().map(from_json).transpose()?,
            recorded,
            drift: drift.as_deref().map(from_text).transpose()?,
        });
    }
    let presets = presets::table
        .order(presets::position)
        .load::<(String, String, i64, Option<String>, String)>(connection)
        .map_err(storage)?
        .into_iter()
        .map(|(id, name, position, color, icon)| {
            Ok(Preset {
                id: PresetId(id),
                name,
                position: position as u32,
                color: color.as_deref().map(from_text).transpose()?,
                icon: from_text(&icon)?,
            })
        })
        .collect::<AppResult<_>>()?;
    let folders = library_folders::table
        .order(library_folders::position)
        .load::<(String, String, i64)>(connection)
        .map_err(storage)?
        .into_iter()
        .map(|(id, name, position)| Folder {
            id: FolderId(id),
            name,
            position: position as u32,
        })
        .collect();
    let library = Library {
        presets,
        folders,
        skills,
    };
    Ok((library, details))
}

/// Replaces the presets, memberships, folders, and filing with those of `library`.
// ponytail: rewrites all organization rows per change; targeted updates if libraries grow large.
pub(crate) fn save_organization(
    connection: &mut SqliteConnection,
    library: &Library,
) -> AppResult<()> {
    let storage = |_| AppError::storage();
    diesel::delete(presets::table)
        .execute(connection)
        .map_err(storage)?;
    diesel::update(skills::table)
        .set(skills::folder_id.eq(None::<String>))
        .execute(connection)
        .map_err(storage)?;
    diesel::delete(library_folders::table)
        .execute(connection)
        .map_err(storage)?;
    for preset in &library.presets {
        diesel::insert_into(presets::table)
            .values((
                presets::id.eq(preset.id.as_str()),
                presets::name.eq(&preset.name),
                presets::position.eq(i64::from(preset.position)),
                presets::color.eq(preset.color.as_ref().map(to_text).transpose()?),
                presets::icon.eq(to_text(&preset.icon)?),
            ))
            .execute(connection)
            .map_err(storage)?;
    }
    for folder in &library.folders {
        diesel::insert_into(library_folders::table)
            .values((
                library_folders::id.eq(folder.id.as_str()),
                library_folders::name.eq(&folder.name),
                library_folders::position.eq(i64::from(folder.position)),
            ))
            .execute(connection)
            .map_err(storage)?;
    }
    for skill in &library.skills {
        for preset in &skill.preset_ids {
            diesel::insert_into(preset_skills::table)
                .values((
                    preset_skills::preset_id.eq(preset.as_str()),
                    preset_skills::slug.eq(skill.id.as_str()),
                ))
                .execute(connection)
                .map_err(storage)?;
        }
        if let Some(folder) = &skill.folder_id {
            diesel::update(skills::table.find(skill.id.as_str()))
                .set(skills::folder_id.eq(folder.as_str()))
                .execute(connection)
                .map_err(storage)?;
        }
    }
    Ok(())
}

fn to_json(value: &impl Serialize) -> AppResult<String> {
    serde_json::to_string(value).map_err(|_| AppError::storage())
}
fn from_json<T: DeserializeOwned>(json: &str) -> AppResult<T> {
    serde_json::from_str(json).map_err(|_| AppError::storage())
}
/// A unit-variant or string-valued enum, such as a preset color or icon, as its bare name.
fn to_text(value: &impl Serialize) -> AppResult<String> {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(text)) => Ok(text),
        _ => Err(AppError::storage()),
    }
}
fn from_text<T: DeserializeOwned>(text: &str) -> AppResult<T> {
    serde_json::from_value(serde_json::Value::String(text.to_owned()))
        .map_err(|_| AppError::storage())
}
