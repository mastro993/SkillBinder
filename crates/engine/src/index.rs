use crate::{library::payload_relative, payload, persistence, worker::State};
use rusqlite::{OptionalExtension, params};
use skillbinder_proto::{AppError, AppResult, SkillDetails};

impl State {
    pub(crate) fn refresh_index(&self) -> AppResult<()> {
        let library = self.read_library()?;
        for skill in &library.skills {
            let cache_key = format!("indexed_digest:{}", skill.id);
            if self.setting::<String>(&cache_key)?.as_deref() == Some(&skill.digest) {
                continue;
            }
            let inspection = payload::inspect(&self.contained_skill_path(
                &self.library_dir().join(payload_relative(&library, skill)),
            )?)?;
            let existing = self
                .database
                .query_row(
                    "SELECT validation FROM skill_metadata WHERE skill_id=?1",
                    [skill.id.as_str()],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(|_| AppError::storage())?;
            let mut details: SkillDetails = existing
                .map(|json| serde_json::from_str(&json).map_err(|_| AppError::storage()))
                .transpose()?
                .unwrap_or_default();
            details.description = inspection.description;
            details.validation = inspection.status;
            details.messages = inspection.messages;
            self.database.execute("INSERT INTO skill_metadata(skill_id,description,validation,updated_at) VALUES(?1,?2,?3,?4) ON CONFLICT(skill_id) DO UPDATE SET description=excluded.description,validation=excluded.validation,updated_at=excluded.updated_at", params![skill.id.as_str(), details.description, serde_json::to_string(&details).map_err(|_| AppError::storage())?, persistence::now()]).map_err(|_| AppError::storage())?;
            self.set_setting(&cache_key, &skill.digest)?;
        }
        Ok(())
    }
}
