//! The whole job over `fixtures/catalog/`: local repositories behind the curated URLs and saved
//! responses for the site, skills.sh, and GitHub, so no test touches the network. The site it
//! builds is kept in `fixtures/catalog/site/`, the v1 sample the app's catalog tests read; set
//! `UPDATE_CATALOG_FIXTURE=1` to rewrite it after an intended change.

#![expect(
    clippy::expect_used,
    reason = "Test assertions report fixture failures."
)]

use super::{
    Job, Outcome, SITE, counts, curation,
    format::{CATALOG, Catalog, HISTORY, check_site},
    http::{Fetched, Http},
};
use anyhow::{anyhow, bail};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

/// 2026-11-19T03:12:44Z.
const NOW: u64 = 1_795_057_964;

enum Reply {
    Found(Vec<u8>),
    Missing,
    Fail,
}

/// Saved responses by URL; any other URL fails.
struct Saved(BTreeMap<String, Reply>);

impl Http for Saved {
    fn get(&self, url: &str, _limit: u64) -> anyhow::Result<Fetched> {
        match self.0.get(url) {
            Some(Reply::Found(bytes)) => Ok(Fetched::Found(bytes.clone())),
            Some(Reply::Missing) => Ok(Fetched::Missing),
            Some(Reply::Fail) => bail!("GET {url} answered HTTP 503"),
            None => Err(anyhow!("GET {url} could not connect")),
        }
    }
}

struct World {
    fixtures: PathBuf,
    /// Holds the bare repositories behind the curated URLs.
    remotes: tempfile::TempDir,
    replies: BTreeMap<String, Reply>,
}

impl World {
    /// The fixture repositories except `unreachable`, with the live site never published.
    fn new(unreachable: &[&str]) -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let fixtures = root.join("fixtures/catalog");
        let remotes = tempfile::tempdir().expect("temporary directory");
        for (owner, name) in [("acme", "skills"), ("acme", "tools"), ("solo", "notes")] {
            if unreachable.contains(&name) {
                continue;
            }
            let tree = fixtures.join("repositories").join(owner).join(name);
            let bare = remotes.path().join(owner).join(format!("{name}.git"));
            crate::fixture::commit_snapshots(&bare, &[tree]).expect("fixture repository");
        }
        let mut world = Self {
            fixtures,
            remotes,
            replies: BTreeMap::new(),
        };
        world.reply(HISTORY, Reply::Missing);
        world.reply(CATALOG, Reply::Missing);
        for (owner, name) in [("acme", "skills"), ("acme", "tools"), ("solo", "notes")] {
            let saved = world.saved(&format!("skills-sh-{owner}-{name}.json"));
            world.replies.insert(search(owner, name), saved);
        }
        let avatar = world.saved("avatar-acme.png");
        world.replies.insert(avatar_url("acme"), avatar);
        world.replies.insert(avatar_url("solo"), Reply::Missing);
        world
    }

    fn saved(&self, name: &str) -> Reply {
        Reply::Found(fs::read(self.fixtures.join("responses").join(name)).expect("saved response"))
    }

    /// Sets what the live site answers for `path`.
    fn reply(&mut self, path: &str, reply: Reply) {
        self.replies.insert(format!("{SITE}{path}"), reply);
    }

    /// Runs the job into a new directory.
    fn build(self) -> (anyhow::Result<Outcome>, tempfile::TempDir) {
        let curated = fs::read_to_string(self.fixtures.join("curated.toml")).expect("curation");
        let base = format!("file://{}/", self.remotes.path().display());
        let job = Job {
            creators: curation::parse(&curated).expect("valid curation"),
            agent_dirs: vec![".claude/skills".to_owned()],
            http: &Saved(self.replies),
            git_config: vec![format!("url.{base}.insteadOf=https://github.com/")],
            now: SystemTime::UNIX_EPOCH + Duration::from_secs(NOW),
        };
        let out = tempfile::tempdir().expect("temporary directory");
        (job.build(out.path()), out)
    }
}

fn search(owner: &str, name: &str) -> String {
    let text = format!(
        "[[creator]]\nid = \"x\"\nname = \"X\"\ngithub = \"x\"\n\
         repositories = [\"https://github.com/{owner}/{name}.git\"]\n"
    );
    let mut creators = curation::parse(&text).expect("valid curation");
    counts::url(counts::BASE, &creators.remove(0).repositories.remove(0))
}

fn avatar_url(login: &str) -> String {
    format!("https://github.com/{login}.png?size=64")
}

fn read_catalog(dir: &Path) -> Catalog {
    serde_json::from_slice(&fs::read(dir.join(CATALOG)).expect("catalog")).expect("catalog v1")
}

/// The relative path and bytes of every file under `dir`.
fn files(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut found = BTreeMap::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in fs::read_dir(&next).expect("readable directory") {
            let path = entry.expect("directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let relative = path.strip_prefix(dir).expect("under dir");
                let name = relative.to_string_lossy().replace('\\', "/");
                found.insert(name, fs::read(&path).expect("readable file"));
            }
        }
    }
    found
}

#[test]
fn the_built_site_matches_the_shared_fixture() {
    let mut world = World::new(&[]);
    world.reply(HISTORY, world.saved("history.json"));
    let shared = world.fixtures.join("site");
    let (outcome, out) = world.build();
    assert!(matches!(outcome.expect("built"), Outcome::Fresh));
    check_site(out.path()).expect("valid site");
    let built = files(out.path());
    if std::env::var_os("UPDATE_CATALOG_FIXTURE").is_some() {
        let _ = fs::remove_dir_all(&shared);
        for (name, bytes) in &built {
            let path = shared.join(name);
            fs::create_dir_all(path.parent().expect("parent")).expect("fixture directory");
            fs::write(path, bytes).expect("fixture file");
        }
    }
    assert!(
        built == files(&shared),
        "The built site differs from fixtures/catalog/site; rerun with UPDATE_CATALOG_FIXTURE=1 if the change is intended"
    );
    let catalog = read_catalog(out.path());
    let order: Vec<(&str, u64)> = catalog
        .skills
        .iter()
        .map(|skill| (skill.slug.as_str(), skill.installs_week))
        .collect();
    assert_eq!(
        order,
        [("tdd", 300), ("lint", 15), ("plan", 10), ("notes", 0)]
    );
    let creators: Vec<_> = catalog
        .creators
        .iter()
        .map(|c| (&*c.id, c.avatar.as_deref()))
        .collect();
    assert_eq!(
        creators,
        [("acme", Some("avatars/acme.png")), ("solo", None)]
    );
}

#[test]
fn a_first_run_starts_history_and_trends_from_zero() {
    let (outcome, out) = World::new(&[]).build();
    assert!(matches!(outcome.expect("built"), Outcome::Fresh));
    let catalog = read_catalog(out.path());
    assert!(catalog.skills.iter().all(|skill| skill.installs_week == 0));
    let history = fs::read_to_string(out.path().join(HISTORY)).expect("history");
    assert!(history.contains("\"2026-11-19\"") && history.contains("#skills/tdd\": 1300"));
}

#[test]
fn a_failed_history_download_stops_the_build() {
    for path in [HISTORY, CATALOG] {
        let mut world = World::new(&[]);
        world.reply(path, Reply::Fail);
        let (outcome, out) = world.build();
        assert!(outcome.is_err(), "built without {path}");
        assert!(files(out.path()).is_empty(), "wrote a site without {path}");
    }
}

#[test]
fn without_skills_sh_the_last_counts_are_published_and_the_run_fails() {
    let mut world = World::new(&[]);
    let published = world.fixtures.join("site");
    world.reply(
        HISTORY,
        Reply::Found(fs::read(published.join(HISTORY)).expect("history")),
    );
    world.reply(
        CATALOG,
        Reply::Found(fs::read(published.join(CATALOG)).expect("catalog")),
    );
    world.replies.insert(search("acme", "tools"), Reply::Fail);
    let (outcome, out) = world.build();
    assert!(matches!(outcome.expect("built"), Outcome::StaleCounts(_)));
    check_site(out.path()).expect("valid site");
    let before = read_catalog(&published);
    let after = read_catalog(out.path());
    let weeks = |catalog: &Catalog| {
        let mut weeks: Vec<_> = catalog
            .skills
            .iter()
            .map(|skill| (skill.path.clone(), skill.installs_week))
            .collect();
        weeks.sort();
        weeks
    };
    assert_eq!(weeks(&after), weeks(&before));
    let history = fs::read(out.path().join(HISTORY)).expect("history");
    assert_eq!(history, fs::read(published.join(HISTORY)).expect("history"));
}

#[test]
fn an_unreachable_repository_keeps_its_last_listing() {
    let mut world = World::new(&["tools"]);
    let published = world.fixtures.join("site");
    world.reply(
        CATALOG,
        Reply::Found(fs::read(published.join(CATALOG)).expect("catalog")),
    );
    let (outcome, out) = world.build();
    assert!(matches!(outcome.expect("built"), Outcome::Fresh));
    let catalog = read_catalog(out.path());
    let lint = catalog
        .skills
        .iter()
        .find(|skill| skill.slug == "lint")
        .expect("lint kept from the last catalog");
    assert_eq!(lint.path, ".claude/skills/lint");
    assert_eq!(lint.repository, "https://github.com/acme/tools.git");
}
