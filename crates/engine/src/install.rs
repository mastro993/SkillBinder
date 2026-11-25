//! Installing a skill from Explore (ADR 0015, design T8): its network phase fetches the skill's
//! folder on the Git pool, never the worker, and a short commit on the worker puts it in the
//! library at `skills/<slug>` with its record and source. The skill is inactive: no preset or
//! workspace holds it, and nothing is written to your agents or workspaces.

mod installs;

pub(crate) use installs::{Begun, Installs};

use crate::{
    git::Git,
    library::normalized,
    library_store::SavedSkill,
    logging::{Area, Level, Logger},
    payload::{self, Inspection},
    persistence::{Journal, Move, RecordChange},
    records::Record,
    shutdown::Shutdown,
    sources::{self, Refusal},
    worker::State,
};
use skillbinder_proto::{
    AppError, CatalogSkill, ErrorCategory, InstallFailure, SkillId, SkillSource,
};
use std::{fs, path::PathBuf};

/// Why an install put nothing in the library.
#[derive(Debug)]
pub(crate) enum Failed {
    /// The library holds the slug, or came to while the install ran; Explore shows the skill as
    /// in library or colliding instead.
    Held,
    /// The application is quitting.
    Cancelled,
    /// Explore shows why on the skill.
    Failure(InstallFailure),
}

impl From<AppError> for Failed {
    fn from(error: AppError) -> Self {
        Self::Failure(match error.category {
            ErrorCategory::Cancelled => return Self::Cancelled,
            ErrorCategory::Offline => InstallFailure::Offline,
            // Git could reach the host, which refused the repository or the folder's files.
            ErrorCategory::Git => InstallFailure::Access,
            _ => InstallFailure::Other(error.message),
        })
    }
}

impl From<Refusal> for InstallFailure {
    fn from(refusal: Refusal) -> Self {
        match refusal {
            Refusal::Blocked => Self::Blocked,
            Refusal::Invalid => Self::Invalid,
            Refusal::Empty => Self::Empty,
        }
    }
}

impl Failed {
    /// The error the install's caller sees; Explore shows the install's state instead.
    pub(crate) fn error(self, slug: &str) -> AppError {
        match self {
            Self::Held => {
                AppError::stale(format!("Your library already has a skill named {slug}."))
            }
            Self::Cancelled => Shutdown::error(),
            Self::Failure(_) => AppError::validation(format!("Couldn't install {slug}.")),
        }
    }
}

/// An install's network phase: everything it needs off the worker.
pub(crate) struct Fetch {
    pub git: Git,
    pub source: SkillSource,
    pub slug: String,
    /// The install's own staging directory; the fetched skill is staged under it.
    pub staging: PathBuf,
    /// The install's own cache repository.
    pub cache: PathBuf,
    pub shutdown: Shutdown,
    pub log: Logger,
}

impl Fetch {
    /// Fetches the skill's folder and stages it for the commit, as `<staging>/new/<slug>`. Its
    /// cache is removed either way, and on failure its staging directory too.
    pub(crate) fn run(self) -> Result<Inspection, Failed> {
        let fetched = self.stage();
        let _ = fs::remove_dir_all(&self.cache);
        if let Err(failed) = &fetched {
            let _ = fs::remove_dir_all(&self.staging);
            if !matches!(failed, Failed::Cancelled) {
                self.log
                    .event(Level::Error, Area::Install, "fetch failed")
                    .field("slug", &self.slug)
                    .field("cause", format_args!("{failed:?}"));
            }
        }
        fetched
    }

    fn stage(&self) -> Result<Inspection, Failed> {
        if self.shutdown.cancelled() {
            return Err(Failed::Cancelled);
        }
        self.git.fetch_source(&self.cache, &self.source)?;
        let into = self.staging.join("source");
        let fetched = sources::checkout(&self.git, &self.cache, &self.source, &into, &self.slug)?
            .ok_or(Failed::Failure(InstallFailure::NotFound))?;
        if let Some(refusal) = sources::refusal(&fetched) {
            return Err(Failed::Failure(refusal.into()));
        }
        // Only the inspected files reach the library, as with an update.
        let directory = self.staging.join("new").join(&self.slug);
        fs::create_dir_all(self.staging.join("new")).map_err(|_| AppError::storage())?;
        payload::materialize(&fetched, &directory)?;
        let staged = payload::inspect(&directory)?;
        if self.shutdown.cancelled() {
            return Err(Failed::Cancelled);
        }
        Ok(staged)
    }
}

impl State {
    /// Refuses an install that cannot start, and otherwise hands over the Git it fetches with.
    pub(crate) fn prepare_install(&self, skill: &CatalogSkill) -> Result<Git, Failed> {
        self.require_recovered()?;
        let git = self.git.clone().ok_or_else(|| {
            Failed::Failure(InstallFailure::Other("Installing needs Git.".into()))
        })?;
        if !self.onboarding_finished()? {
            return Err(AppError::validation("Finish onboarding before installing skills.").into());
        }
        // The slug names the directory the skill is staged in and installed to.
        if !payload::portable(&skill.slug) {
            return Err(
                AppError::validation("This skill's name is not a usable directory name.").into(),
            );
        }
        self.refuse_held(&skill.slug).map(|()| git)
    }

    /// Puts the skill staged at `<staging>/new/<slug>` in the library, in one journaled change,
    /// unless the library came to hold its slug while it was fetched. The staging directory is
    /// removed afterwards unless recovery needs it.
    pub(crate) fn commit_install(
        &mut self,
        slug: String,
        source: SkillSource,
        staging: PathBuf,
        staged: &Inspection,
    ) -> Result<(), Failed> {
        let committed = self.commit_staged(&slug, &source, &staging, staged);
        if !self.recovery_required {
            let _ = fs::remove_dir_all(&staging);
        }
        if let Err(failed) = &committed
            && !matches!(failed, Failed::Cancelled)
        {
            self.log
                .event(Level::Error, Area::Install, "install failed")
                .field("slug", &slug)
                .field("cause", format_args!("{failed:?}"));
        }
        committed
    }

    /// `source` has no pinned ref, so the skill follows the repository's default branch and
    /// Update works on it.
    fn commit_staged(
        &mut self,
        slug: &str,
        source: &SkillSource,
        staging: &std::path::Path,
        staged: &Inspection,
    ) -> Result<(), Failed> {
        self.require_recovered()?;
        self.refuse_held(slug)?;
        let record = Record::of(staged, Some(source.clone()));
        self.commit(Journal {
            id: format!("install-{}", uuid::Uuid::new_v4()),
            moves: vec![Move {
                from: staging.join("new").join(slug),
                to: self.library_dir().join("skills").join(slug),
            }],
            completed_moves: 0,
            progress: Default::default(),
            // `imported_at` holds when the skill arrived in this library. Only install marks a
            // skill New.
            skills: vec![SavedSkill {
                new: true,
                ..SavedSkill::new(
                    slug.to_owned(),
                    staged,
                    Some(crate::lifecycle::format_timestamp()),
                    record.clone(),
                    None,
                )
            }],
            outcome: None,
            records: vec![RecordChange {
                slug: slug.to_owned(),
                record: Some(record),
                previous: None,
            }],
            held: Vec::new(),
        })?;
        self.updates.current_with(
            SkillId(slug.to_owned()),
            source.clone(),
            staged.tree.clone(),
        );
        Ok(())
    }

    /// Refuses a slug the library holds, compared as the library compares names, including a
    /// directory not yet reconciled.
    fn refuse_held(&self, slug: &str) -> Result<(), Failed> {
        if self.held_names()?.contains(&normalized(slug)) {
            Err(Failed::Held)
        } else {
            Ok(())
        }
    }
}
