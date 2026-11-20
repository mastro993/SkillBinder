//! The catalog job (ADR 0014, `docs/specs/explore-and-install/catalog-job.md`): lists the
//! curated creators' skills, reads their installs from skills.sh, extends the history the live
//! site published, and builds catalog format v1 with the creators' avatars for GitHub Pages.

mod counts;
mod curation;
mod date;
mod discovery;
mod format;
mod frontmatter;
mod history;
mod http;
mod listing;
mod rules;
#[cfg(test)]
mod tests;
mod updates;

pub(crate) use date::timestamp;

use anyhow::{Context as _, bail, ensure};
use curation::Creator;
use date::Day;
use format::{CATALOG, Catalog, HISTORY, Skill};
use history::History;
use http::{Fetched, Http};
use std::{collections::BTreeMap, fs, path::Path, time::SystemTime};

/// The GitHub Pages site the catalog publishes to; the app compiles in the same base URL.
const SITE: &str = "https://mastro993.github.io/SkillBinder/";
const AVATAR_BYTES: u64 = 256 * 1024;
const PREVIOUS_BYTES: u64 = 16 * 1024 * 1024;

/// How a build ended when it wrote the site.
pub(crate) enum Outcome {
    Fresh,
    /// skills.sh failed: the site carries fresh listings with the last published counts, and
    /// the run must fail once it is deployed.
    StaleCounts(anyhow::Error),
    /// The releases could not be read: the site carries the last published update manifests,
    /// and the run must fail once it is deployed.
    StaleUpdates(anyhow::Error),
}

/// `catalog --out <dir>` builds the site, `catalog updates --out <dir>` adds the update manifests
/// to it, and `catalog check <dir>` checks a built one.
pub(crate) fn run(root: &Path, args: &[String]) -> anyhow::Result<Outcome> {
    match args {
        [flag, out] if flag == "--out" => {
            let curated = fs::read_to_string(root.join("catalog/curated.toml"))?;
            let job = Job {
                creators: curation::parse(&curated)?,
                agent_dirs: crate::registry::workspace_skill_dirs(root)?,
                http: &http::Web::new(),
                git_config: Vec::new(),
                now: SystemTime::now(),
            };
            job.build(Path::new(out))
        }
        [command, flag, out] if command == "updates" && flag == "--out" => {
            updates::mirror(&updates::Gh, &http::Web::new(), Path::new(out))
        }
        [command, dir] if command == "check" => {
            format::check_site(Path::new(dir))?;
            println!("{dir} holds a valid catalog format v1 site.");
            Ok(Outcome::Fresh)
        }
        _ => bail!(
            "Usage: cargo run -p xtask -- catalog <--out <directory>|updates --out <directory>|check <directory>>"
        ),
    }
}

struct Job<'a> {
    creators: Vec<Creator>,
    /// The registry's workspace skill directories, which discovery searches.
    agent_dirs: Vec<String>,
    http: &'a dyn Http,
    /// Extra Git `name=value` settings; tests map the curated URLs to local repositories.
    git_config: Vec<String>,
    now: SystemTime,
}

impl Job<'_> {
    fn build(&self, out: &Path) -> anyhow::Result<Outcome> {
        if out.exists() {
            ensure!(
                fs::read_dir(out)?.next().is_none(),
                "{} is not empty; choose a new directory",
                out.display()
            );
        }
        let mut history = match self.previous(HISTORY)? {
            Some(bytes) => History::parse(&bytes)?,
            None => History::default(),
        };
        let previous: Option<Catalog> = self
            .previous(CATALOG)?
            .map(|bytes| serde_json::from_slice(&bytes))
            .transpose()
            .context("The published catalog is malformed")?;
        let previous = previous.map(|catalog| catalog.skills).unwrap_or_default();
        let mut skills = self.list(&previous)?;
        let today = Day::of(self.now)?;
        let stale = match self.totals(&skills) {
            Ok(totals) => {
                history.record(today, totals);
                for skill in &mut skills {
                    let key = history::key(&skill.repository, &skill.path);
                    skill.installs_week = history.gain(today, &key);
                }
                None
            }
            Err(error) => {
                let weeks: BTreeMap<_, _> = previous
                    .iter()
                    .map(|skill| ((&skill.repository, &skill.path), skill.installs_week))
                    .collect();
                for skill in &mut skills {
                    let key = (&skill.repository, &skill.path);
                    skill.installs_week = weeks.get(&key).copied().unwrap_or_default();
                }
                Some(error)
            }
        };
        skills.sort_by(|a, b| {
            (b.installs_week, &a.repository, &a.path).cmp(&(
                a.installs_week,
                &b.repository,
                &b.path,
            ))
        });
        let catalog = Catalog {
            schema: format::SCHEMA,
            generated_at: date::timestamp(self.now)?,
            creators: self.creators(&skills, out)?,
            skills,
        };
        let problems = format::violations(&catalog);
        ensure!(
            problems.is_empty(),
            "The built catalog breaks format v1:\n{}",
            problems.join("\n")
        );
        write_json(&out.join(CATALOG), &catalog)?;
        write_json(&out.join(HISTORY), &history)?;
        println!(
            "Built {} skills from {} creators with {} nights of history into {}.",
            catalog.skills.len(),
            catalog.creators.len(),
            history.nights(),
            out.display()
        );
        Ok(stale.map_or(Outcome::Fresh, Outcome::StaleCounts))
    }

    /// A file the live site published, or `None` when it answers 404: the first run, or a lost
    /// site. Any other failure stops the build, so the previous site stays up.
    fn previous(&self, path: &str) -> anyhow::Result<Option<Vec<u8>>> {
        let url = format!("{SITE}{path}");
        match self.http.get(&url, PREVIOUS_BYTES)? {
            Fetched::Found(bytes) => Ok(Some(bytes)),
            Fetched::Missing => Ok(None),
        }
    }

    /// Every curated skill, with no installs yet. A repository that cannot be listed keeps its
    /// entries from the `previous` catalog.
    fn list(&self, previous: &[Skill]) -> anyhow::Result<Vec<Skill>> {
        let scratch = tempfile::tempdir()?;
        let mut skills = Vec::new();
        for creator in &self.creators {
            for repository in &creator.repositories {
                let listed = listing::list(
                    repository,
                    &self.agent_dirs,
                    &self.git_config,
                    scratch.path(),
                );
                let error = match listed {
                    Ok(listed) => {
                        skills.extend(listed.into_iter().map(|listed| Skill {
                            name: listed.slug.clone(),
                            slug: listed.slug,
                            description: listed.description,
                            creator: creator.id.clone(),
                            repository: repository.url.clone(),
                            path: listed.path,
                            installs_week: 0,
                        }));
                        continue;
                    }
                    Err(error) => error,
                };
                println!(
                    "::warning::Kept the last listing of {}: {error:#}",
                    repository.url
                );
                skills.extend(
                    previous
                        .iter()
                        .filter(|skill| {
                            skill.creator == creator.id
                                && skill.repository == repository.url
                                && !repository.excluded.contains(&skill.path)
                        })
                        .cloned(),
                );
            }
        }
        Ok(skills)
    }

    /// Tonight's all-time installs of `skills`, by history key. A skill skills.sh does not list
    /// is left out, so a night it drops out of the search never becomes its trending baseline;
    /// it trends 0 until it is counted.
    fn totals(&self, skills: &[Skill]) -> anyhow::Result<history::Totals> {
        let mut totals = history::Totals::new();
        for repository in self.creators.iter().flat_map(|c| &c.repositories) {
            let listed: Vec<&Skill> = skills
                .iter()
                .filter(|skill| skill.repository == repository.url)
                .collect();
            if listed.is_empty() {
                continue;
            }
            let counts = counts::fetch(self.http, counts::BASE, repository)?;
            for skill in listed {
                if let Some(&installs) = counts.get(&skill.slug) {
                    totals.insert(history::key(&skill.repository, &skill.path), installs);
                }
            }
        }
        Ok(totals)
    }

    /// The creators that have skills, in curated order, with their avatars mirrored into
    /// `out`. A creator whose avatar cannot be fetched shows initials.
    fn creators(&self, skills: &[Skill], out: &Path) -> anyhow::Result<Vec<format::Creator>> {
        let mut creators = Vec::new();
        for creator in &self.creators {
            if !skills.iter().any(|skill| skill.creator == creator.id) {
                continue;
            }
            let url = format!("https://github.com/{}.png?size=64", creator.github);
            let avatar = match self.http.get(&url, AVATAR_BYTES) {
                Ok(Fetched::Found(image)) => match format::Image::of(&image) {
                    Some(kind) => {
                        let path = format::avatar(&creator.id, kind);
                        write(&out.join("v1").join(&path), &image)?;
                        Some(path)
                    }
                    None => {
                        println!("::warning::{url} is neither a PNG nor a JPEG");
                        None
                    }
                },
                Ok(Fetched::Missing) => {
                    println!("::warning::{url} answered HTTP 404");
                    None
                }
                Err(error) => {
                    println!("::warning::No avatar for {}: {error:#}", creator.id);
                    None
                }
            };
            creators.push(format::Creator {
                id: creator.id.clone(),
                name: creator.name.clone(),
                avatar,
            });
        }
        Ok(creators)
    }
}

fn write_json(path: &Path, value: &impl serde::Serialize) -> anyhow::Result<()> {
    write(
        path,
        (serde_json::to_string_pretty(value)? + "\n").as_bytes(),
    )
}

fn write(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes).with_context(|| format!("Could not write {}", path.display()))
}
