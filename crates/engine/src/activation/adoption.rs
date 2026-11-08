//! The library skills your agents already have become active once (ADR 0006): the first time the
//! library holds skills after onboarding, when onboarding finishes, when an import, Pull, or Sync
//! fills an empty library, or at a launch for a library from before this, the active preset
//! holds each skill it does not hold yet for the agents whose own global skill directory holds a
//! copy with the library's content, and those copies are adopted. A copy with other content, a
//! link, a Git checkout, and a directory SkillBinder already records for the slug are left as
//! they are. It never runs again on the device, so a copy placed later stays unmanaged.

use super::pass::Scope;
use super::{held_copy, store};
use crate::{database, discovery::registry, library_store, persistence, worker::State};
use skillbinder_proto::{AppResult, OnboardingStep, Targets};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// The device setting recording that the library skills your agents had were adopted.
const ADOPTED: &str = "agent_copies_adopted";

impl State {
    /// Adopts the library skills your agents already have, once, after onboarding and as soon as
    /// the library holds skills. A failure leaves it to the next launch, import, Pull, or Sync.
    pub(crate) fn adopt_agent_copies(&self) {
        if let Err(error) = self.try_adopt_agent_copies() {
            self.log.record(format!(
                "Your agents' skills will be adopted later: {error}"
            ));
        }
    }

    fn try_adopt_agent_copies(&self) -> AppResult<()> {
        if self.recovery_required
            || self.setting::<bool>(ADOPTED)?.unwrap_or(false)
            || self
                .setting::<OnboardingStep>("onboarding")?
                .unwrap_or_default()
                != OnboardingStep::Complete
        {
            return Ok(());
        }
        let library = self.read_library()?;
        if library.skills.is_empty() {
            return Ok(());
        }
        let agents = registry::agent_options(&self.config)?;
        let recorded = store::load(&mut self.database.borrow_mut())?;
        // Each directory's copy of a slug, probed once however many agents share the directory.
        let mut probed = BTreeMap::<(&str, &str), Option<String>>::new();
        let mut held = BTreeMap::<String, BTreeSet<String>>::new();
        for skill in &library.skills {
            // A quit before every copy was probed leaves the adoption to the next launch.
            if self.shutdown.cancelled() {
                return Ok(());
            }
            if skill.presets.contains_key(&library.active_preset) {
                continue;
            }
            let slug = skill.id.as_str();
            let mut content = None;
            for agent in agents.iter().filter(|agent| agent.detected) {
                let Some(directory) = agent.resolved_skills_dir.as_deref() else {
                    continue;
                };
                if self.protected(Path::new(directory))
                    || recorded.contains_key(&(directory.to_owned(), slug.to_owned()))
                {
                    continue;
                }
                let copy = probed
                    .entry((directory, slug))
                    .or_insert_with(|| held_copy(&Path::new(directory).join(slug)));
                let Some(copy) = copy else {
                    continue;
                };
                let digest = content.get_or_insert_with(|| {
                    self.library_content(slug)
                        .ok()
                        .map(|source| source.manifest.digest)
                });
                if digest.as_ref() == Some(copy) {
                    held.entry(slug.to_owned())
                        .or_default()
                        .insert(agent.id.clone());
                }
            }
        }
        database::transaction(&mut self.database.borrow_mut(), |connection| {
            for (slug, agents) in &held {
                let targets = Targets::Agents(agents.clone());
                library_store::hold_in_preset(connection, &library.active_preset, slug, &targets)?;
            }
            persistence::put_setting(connection, ADOPTED, "true")
        })?;
        self.quietly(Scope::Slugs(held.into_keys().collect()));
        Ok(())
    }
}
