use crate::{database, library::payload_relative, payload, worker::State};
use skillbinder_proto::AppResult;

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
            let mut details =
                database::details(&mut self.database.borrow_mut(), &skill.id)?.unwrap_or_default();
            details.description = inspection.description;
            details.validation = inspection.status;
            details.messages = inspection.messages;
            database::save_details(&mut self.database.borrow_mut(), &skill.id, &details)?;
            self.set_setting(&cache_key, &skill.digest)?;
        }
        Ok(())
    }
}
