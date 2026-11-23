//! Finding skills and bringing them into the library: Explore's search and Install, Scan for
//! skills and its import, and your repositories.

use super::{Agreed, ResponseFormat, required, usage};
use schemars::JsonSchema;
use serde::Deserialize;
use skillbinder_proto::command_line::{Change, CommandError, Find, Request};

/// Where find_skills looks.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FindWhere {
    /// Searches the catalog, the user's repositories, and skills.sh for `query`, as Explore
    /// does; install a result with install_skills.
    Explore,
    /// Scans the user's agents and workspaces now and lists the skills found there that are
    /// not in the library, with their versions; import them with import_skills.
    Found,
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct FindSkills {
    /// Where to look.
    #[serde(rename = "where")]
    place: FindWhere,
    /// What to search for, for explore.
    query: Option<String>,
    #[serde(default)]
    pub(crate) response_format: ResponseFormat,
}

impl FindSkills {
    pub(crate) fn request(&self) -> Result<Request, CommandError> {
        let find = match self.place {
            FindWhere::Explore => Find::Search {
                query: required(&self.query, "query")?,
            },
            FindWhere::Found => Find::Scan,
        };
        Ok(Request::Find { find })
    }
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct InstallSkills {
    /// The skills, each as find_skills' reference: `<owner>/<repo>/<slug>`, skills.sh's
    /// `<owner>/<repo>@<skill>`, or a slug found once in the catalog or the user's
    /// repositories.
    refs: Vec<String>,
}

impl InstallSkills {
    pub(crate) fn change(&self) -> Result<Change, CommandError> {
        if self.refs.is_empty() {
            return Err(usage("install_skills needs refs."));
        }
        Ok(Change::Install {
            skills: self.refs.clone(),
        })
    }
}

/// A found skill to import.
#[derive(Deserialize, JsonSchema)]
pub(crate) struct FoundSkill {
    /// Its slug.
    slug: String,
    /// One of its versions as find_skills lists them; needed when it has more than one.
    version: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct ImportSkills {
    /// The found skills to import.
    #[serde(default)]
    skills: Vec<FoundSkill>,
    /// Import every skill the Import skills dialog ticks, instead of `skills`.
    #[serde(default)]
    all: bool,
    /// Replace the user's copies the import replaces, which the result lists when it needs
    /// this. Only after the user agreed.
    #[serde(default)]
    replace: bool,
}

impl ImportSkills {
    pub(crate) fn change(&self) -> Result<Change, CommandError> {
        if self.all != self.skills.is_empty() {
            return Err(usage("Name skills or pass all, not both."));
        }
        let skills = self.skills.iter().map(|skill| match &skill.version {
            Some(version) => format!("{}@{version}", skill.slug),
            None => skill.slug.clone(),
        });
        Ok(Change::Import {
            skills: skills.collect(),
            all: self.all,
        })
    }

    pub(crate) fn agreed(&self) -> Agreed {
        Agreed {
            replace: self.replace,
            confirm: false,
        }
    }
}

/// What manage_repositories does.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RepositoryAction {
    /// Adds the Git repository at `repository`, a URL, to browse and install its skills.
    Add,
    /// Removes `repository`; the skills installed from it stay in the library.
    Remove,
    /// Installs each of `repository`'s skills the library lacks, inactive.
    InstallAll,
}

#[derive(Deserialize, JsonSchema)]
pub(crate) struct ManageRepositories {
    /// What to do.
    action: RepositoryAction,
    /// The repository: its Git URL, or `<owner>/<repo>` for one already added.
    repository: String,
}

impl ManageRepositories {
    pub(crate) fn change(&self) -> Change {
        let repository = self.repository.clone();
        match self.action {
            RepositoryAction::Add => Change::AddRepository { url: repository },
            RepositoryAction::Remove => Change::RemoveRepository { repository },
            RepositoryAction::InstallAll => Change::InstallAll { repository },
        }
    }
}
