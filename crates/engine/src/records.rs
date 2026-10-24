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

/// The text of the record file at `path`; `None` when it is larger than `MAX_RECORD_BYTES`.
fn read_bounded(path: &std::path::Path) -> std::io::Result<Option<String>> {
    use std::io::Read;
    let file = fs::File::open(path)?;
    if file.metadata()?.len() > MAX_RECORD_BYTES {
        return Ok(None);
    }
    let mut text = String::new();
    file.take(MAX_RECORD_BYTES).read_to_string(&mut text)?;
    Ok(Some(text))
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
        match self.stored_record(slug) {
            Ok(record) => record,
            Err(error) => {
                self.log
                    .record(format!("Record of {slug} is unreadable: {error}"));
                None
            }
        }
    }

    /// The record stored for `slug`: `None` when there is none or it is invalid, which is logged,
    /// and an error when the file cannot be read now, so a caller can retry.
    pub(crate) fn stored_record(&self, slug: &str) -> AppResult<Option<Record>> {
        let text = match read_bounded(&self.record_file(slug)?) {
            Ok(Some(text)) => text,
            Ok(None) => {
                self.log.record(format!("Record of {slug} is too large"));
                return Ok(None);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(AppError::storage()),
        };
        match toml::from_str::<Record>(&text)
            .map_err(|error| error.to_string())
            .and_then(|record| valid(&record).map(|()| record))
        {
            Ok(record) => Ok(Some(record)),
            Err(error) => {
                self.log
                    .record(format!("Record of {slug} is invalid: {error}"));
                Ok(None)
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

    /// Writes each library skill's record whose file is missing or lacks what the database
    /// caches, such as a learned source, before a Push or Sync commits the library.
    pub(crate) fn write_pending_records(&self) -> AppResult<()> {
        for skill in self.read_library()?.skills {
            if skill.recorded.is_empty() {
                continue;
            }
            let slug = skill.id.as_str();
            let record = Record {
                tree: skill.recorded,
                source: skill.source,
            };
            if self.read_record(slug).as_ref() != Some(&record) {
                self.write_record(slug, &record)?;
            }
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
            let entry = entry.map_err(|_| AppError::storage())?;
            let name = entry.file_name();
            let Some(slug) = name.to_str().and_then(|name| name.strip_suffix(".toml")) else {
                continue;
            };
            if listed.contains(slug) {
                continue;
            }
            // Something no import wrote, such as a folder or a name no skill can have, is left
            // for the user rather than failing every reconcile.
            if !payload::portable(slug) || !entry.file_type().is_ok_and(|kind| kind.is_file()) {
                self.log.record(format!(
                    "Leaving an unexpected entry in records: {slug}.toml"
                ));
                continue;
            }
            self.log
                .record(format!("Removing the orphan record of {slug}"));
            self.remove_record(slug)?;
        }
        Ok(())
    }
}
