//! A local Git repository publishing skills under `skills/`, as a catalog entry's source.
#![expect(
    clippy::unwrap_used,
    reason = "Test assertions report fixture and operation failures."
)]
#![allow(dead_code, reason = "Not every test crate uses every helper.")]

use crate::support::write_skill;
use skillbinder_proto::{CatalogOrigin, CatalogSkill, SkillSource};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=Publisher",
            "-c",
            "user.email=p@example.invalid",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}");
}

pub struct Publisher {
    pub dir: PathBuf,
    pub url: String,
}

impl Publisher {
    /// An empty repository at `dir`.
    pub fn new(dir: PathBuf) -> Self {
        fs::create_dir(&dir).unwrap();
        git(&dir, &["init", "--initial-branch=main"]);
        let url = url::Url::from_file_path(&dir).unwrap().into();
        Self { dir, url }
    }

    /// Commits `skills/<folder>` holding a skill named `slug` with `body`.
    pub fn publish(&self, folder: &str, slug: &str, body: &str) {
        let directory = write_skill(&self.dir.join("skills"), folder, body);
        fs::write(
            directory.join("SKILL.md"),
            format!("---\nname: {slug}\ndescription: Test fixture\n---\n{body}\n"),
        )
        .unwrap();
        self.commit(body);
    }

    pub fn commit(&self, message: &str) {
        git(&self.dir, &["add", "-A"]);
        git(&self.dir, &["commit", "-m", message]);
    }

    /// The catalog entry of the skill `slug` in `skills/<folder>`.
    pub fn skill(&self, folder: &str, slug: &str) -> CatalogSkill {
        catalog_skill(&self.url, &format!("skills/{folder}"), slug)
    }
}

pub fn catalog_skill(url: &str, path: &str, slug: &str) -> CatalogSkill {
    CatalogSkill {
        slug: slug.into(),
        key: slug.into(),
        name: slug.into(),
        description: "Test fixture".into(),
        origin: CatalogOrigin::Curated {
            creator: "acme".into(),
        },
        source: SkillSource {
            url: url.into(),
            reference: None,
            path: path.into(),
        },
        installs_week: 0,
    }
}
