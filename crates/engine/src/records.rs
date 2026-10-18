//! Library skills' records, `records/<slug>.toml` beside the skill directories so they sync with
//! them: the Git tree id of the content each skill should have, and its source when known.

use crate::{payload, persistence, worker::State};
use serde::{Deserialize, Serialize};
use skillbinder_proto::{AppError, AppResult, SkillSource};
use std::{collections::BTreeSet, fs, io::Write, path::PathBuf};

/// What a library skill should contain and where it was published.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct Record {
    /// The Git tree id of the skill's content.
    pub tree: String,
    /// Where the skill was published, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SkillSource>,
}

impl Record {
    /// A record of `inspection`'s content.
    pub(crate) fn of(inspection: &payload::Inspection, source: Option<SkillSource>) -> Self {
        Self {
            tree: inspection.tree.clone(),
            source,
        }
    }
}

/// A record holds a tree id and a source; anything larger is not one.
const MAX_RECORD_BYTES: u64 = 64 * 1024;

/// The text of the record file at `path`, refusing one larger than `MAX_RECORD_BYTES`.
fn read_bounded(path: &std::path::Path) -> std::io::Result<String> {
    use std::io::Read;
    let file = fs::File::open(path)?;
    if file.metadata()?.len() > MAX_RECORD_BYTES {
        return Err(std::io::Error::other("the record is too large"));
    }
    let mut text = String::new();
    file.take(MAX_RECORD_BYTES).read_to_string(&mut text)?;
    Ok(text)
}

fn valid(record: &Record) -> Result<(), String> {
    // Lowercase only, as computed tree ids are, so equal trees compare equal.
    if record.tree.len() != 40
        || !record
            .tree
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("its tree id is malformed".to_owned());
    }
    match &record.source {
        Some(source) => crate::git::validate_source(source).map_err(|error| error.message),
        None => Ok(()),
    }
}

impl State {
    fn records_dir(&self) -> PathBuf {
        self.library_dir().join("records")
    }

    fn record_file(&self, slug: &str) -> AppResult<PathBuf> {
        if !payload::portable(slug) {
            return Err(AppError::storage());
        }
        Ok(self.records_dir().join(format!("{slug}.toml")))
    }

    /// The record stored for `slug`, if any. An unreadable or invalid file reads as none, and the
    /// reason is logged.
    pub(crate) fn read_record(&self, slug: &str) -> Option<Record> {
        let text = match read_bounded(&self.record_file(slug).ok()?) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
            Err(error) => {
                self.log
                    .record(format!("Record of {slug} is unreadable: {error}"));
                return None;
            }
        };
        match toml::from_str::<Record>(&text)
            .map_err(|error| error.to_string())
            .and_then(|record| valid(&record).map(|()| record))
        {
            Ok(record) => Some(record),
            Err(error) => {
                self.log
                    .record(format!("Record of {slug} is invalid: {error}"));
                None
            }
        }
    }

    /// Durably writes `slug`'s record, replacing any previous one.
    pub(crate) fn write_record(&self, slug: &str, record: &Record) -> AppResult<()> {
        let path = self.record_file(slug)?;
        let directory = self.records_dir();
        fs::create_dir_all(&directory).map_err(|_| AppError::storage())?;
        let text = toml::to_string(record).map_err(|_| AppError::storage())?;
        // Staged outside the synced folder, so an interrupted write never leaves a file there.
        let mut file = tempfile::NamedTempFile::new_in(self.config.data_dir.join("staging"))
            .map_err(|_| AppError::storage())?;
        file.write_all(text.as_bytes())
            .and_then(|()| file.as_file().sync_all())
            .map_err(|_| AppError::storage())?;
        file.persist(&path).map_err(|_| AppError::storage())?;
        persistence::sync_directory(&directory)
    }

    /// Removes `slug`'s record, if any.
    pub(crate) fn remove_record(&self, slug: &str) -> AppResult<()> {
        match fs::remove_file(self.record_file(slug)?) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(AppError::storage()),
            // Durable before a journal that removed it is deleted.
            Ok(()) => persistence::sync_directory(&self.records_dir()),
        }
    }

    /// Writes `record` as `slug`'s record, or removes it when there is none.
    pub(crate) fn put_record(&self, slug: &str, record: Option<&Record>) -> AppResult<()> {
        match record {
            Some(record) => self.write_record(slug, record),
            None => self.remove_record(slug),
        }
    }

    /// Puts back `slug`'s previous record after a change failed, logging instead of failing.
    pub(crate) fn restore_record(&self, slug: &str, record: Option<&Record>) {
        if let Err(error) = self.put_record(slug, record) {
            self.log
                .record(format!("Record of {slug} could not be restored: {error}"));
        }
    }

    /// Writes the record of each library skill that has none on disk from the one the database
    /// caches, before a Push or Sync commits the library.
    pub(crate) fn write_missing_records(&self) -> AppResult<()> {
        for skill in self.read_library()?.skills {
            let slug = skill.id.as_str();
            if skill.recorded.is_empty() || self.read_record(slug).is_some() {
                continue;
            }
            let record = Record {
                tree: skill.recorded,
                source: skill.source,
            };
            self.write_record(slug, &record)?;
        }
        Ok(())
    }

    /// Removes the records whose skill directory is not among `listed`, such as one an import
    /// interrupted before its skill arrived left behind.
    pub(crate) fn remove_orphan_records(&self, listed: &BTreeSet<String>) -> AppResult<()> {
        let entries = match fs::read_dir(self.records_dir()) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(_) => return Err(AppError::storage()),
        };
        for entry in entries {
            let name = entry.map_err(|_| AppError::storage())?.file_name();
            if let Some(slug) = name.to_str().and_then(|name| name.strip_suffix(".toml"))
                && !listed.contains(slug)
            {
                self.log
                    .record(format!("Removing the orphan record of {slug}"));
                self.remove_record(slug)?;
            }
        }
        Ok(())
    }
}
