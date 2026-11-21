//! An isolated device whose app update site is a fixture directory laid out like the catalog's,
//! signed with a key made for the test. Each published version's archive is
//! `downloads/<version>`, holding `build <version>`, which an installed fixture bundle stages as
//! its executable, a Windows fixture as its setup program, and a Linux fixture as its AppImage.
#![allow(dead_code, reason = "Each test crate uses part of the fixture.")]

use minisign::KeyPair;
use sha2::{Digest, Sha256};
use skillbinder_engine::{CatalogSource, Engine, EngineConfig, UpdatePayload, UpdateSource};
use skillbinder_proto::{AppUpdateState, AppVersion};
use std::{fs, io::Cursor, path::PathBuf};
use tempfile::TempDir;

/// The archive key of the platform the tests run on.
pub const TARGET: &str = if cfg!(target_os = "macos") {
    "universal-apple-darwin"
} else if cfg!(windows) {
    "x86_64-pc-windows-msvc"
} else {
    "x86_64-unknown-linux-gnu"
};

pub struct Fixture {
    _temp: TempDir,
    pub root: PathBuf,
    pub data: PathBuf,
    pub home: PathBuf,
    pub site: PathBuf,
    pub key: KeyPair,
    pub current: AppVersion,
    /// The running bundle, if the app runs from one.
    pub bundle: Option<PathBuf>,
    /// The running Windows executable, if the app runs from one.
    pub executable: Option<PathBuf>,
    /// The running AppImage, if the app runs as one.
    pub appimage: Option<PathBuf>,
    /// What the archives hold.
    pub payload: UpdatePayload,
}

impl Fixture {
    /// A device running `current` from no bundle, as a development build does.
    pub fn new(current: &str) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().to_path_buf();
        Self {
            _temp: temp,
            data: root.join("data"),
            home: root.join("home"),
            site: root.join("site"),
            root,
            key: KeyPair::generate_unencrypted_keypair().unwrap(),
            current: current.parse().unwrap(),
            bundle: None,
            executable: None,
            appimage: None,
            payload: UpdatePayload::Executable,
        }
    }

    /// A device running `current` from `SkillBinder.app` in its Applications folder.
    pub fn installed(current: &str) -> Self {
        let mut fixture = Self::new(current);
        let bundle = fixture.applications().join("SkillBinder.app");
        fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
        fs::write(
            bundle.join("Contents/MacOS/skillbinder"),
            format!("build {current}"),
        )
        .unwrap();
        fixture.bundle = Some(bundle);
        fixture
    }

    /// A Windows device running `current` from `skillbinder.exe` in its programs folder, with
    /// the setup program's uninstaller beside it when `set_up`. Its archives are setup programs.
    pub fn windows(current: &str, set_up: bool) -> Self {
        let mut fixture = Self::new(current);
        let program = fixture.root.join("Programs/SkillBinder");
        fs::create_dir_all(&program).unwrap();
        let executable = program.join("skillbinder.exe");
        fs::write(&executable, format!("build {current}")).unwrap();
        if set_up {
            fs::write(program.join("unins000.exe"), "uninstaller").unwrap();
        }
        fixture.executable = Some(executable);
        fixture.payload = UpdatePayload::Setup;
        fixture
    }

    /// A Linux device running `current` as an AppImage in `~/Applications`. Its archives are
    /// AppImages.
    pub fn appimage(current: &str) -> Self {
        let mut fixture = Self::new(current);
        let folder = fixture.home.join("Applications");
        fs::create_dir_all(&folder).unwrap();
        let appimage = folder.join(format!("skillbinder-{current}-x86_64.AppImage"));
        fs::write(&appimage, format!("build {current}")).unwrap();
        fixture.appimage = Some(appimage);
        fixture.payload = UpdatePayload::AppImage;
        fixture
    }

    pub fn applications(&self) -> PathBuf {
        self.root.join("Applications")
    }

    pub fn config(&self) -> EngineConfig {
        let mut config = EngineConfig::isolated(self.data.clone(), self.home.clone());
        config.app_version = self.current;
        config.placement.bundle = self.bundle.clone();
        config.placement.executable = self.executable.clone();
        config.placement.appimage = self.appimage.clone();
        config.placement.applications = self.applications();
        config.app_update = Some(UpdateSource {
            site: CatalogSource::Directory(self.site.clone()),
            public_key: self.key.pk.to_base64(),
            payload: self.payload.clone(),
        });
        config
    }

    pub fn open(&self) -> Engine {
        Engine::open(self.config()).unwrap()
    }

    /// Publishes `version` on `channel`, with an archive for `target` whose manifest says it has
    /// `sha256`, signed with `key`.
    pub fn publish_with(
        &self,
        channel: &str,
        version: &str,
        target: &str,
        key: &KeyPair,
        sha256: Option<&str>,
    ) {
        let archive = format!("build {version}");
        let sha256 =
            sha256.map_or_else(|| format!("{:x}", Sha256::digest(&archive)), str::to_owned);
        let size = archive.len();
        let manifest = format!(
            r#"{{"version":"{version}","notes_url":"https://example.invalid/v{version}","published_at":"2026-12-01T10:00:00Z","assets":{{"{target}":{{"url":"downloads/{version}","sha256":"{sha256}","size":{size}}}}},"later":true}}"#
        );
        let signature = minisign::sign(None, &key.sk, Cursor::new(&manifest), None, None).unwrap();
        let directory = self.site.join("updates/v1");
        fs::create_dir_all(&directory).unwrap();
        fs::create_dir_all(self.site.join("downloads")).unwrap();
        fs::write(self.site.join("downloads").join(version), archive).unwrap();
        fs::write(directory.join(format!("{channel}.json")), &manifest).unwrap();
        fs::write(
            directory.join(format!("{channel}.json.minisig")),
            signature.into_string(),
        )
        .unwrap();
    }

    pub fn publish(&self, channel: &str, version: &str) {
        self.publish_with(channel, version, TARGET, &self.key, None);
    }

    pub fn log(&self) -> String {
        fs::read_to_string(self.data.join("logs/skillbinder.log")).unwrap_or_default()
    }

    /// The names in the Applications folder, sorted.
    pub fn applications_names(&self) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(self.applications())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// The executable of the bundle at `name` in the Applications folder.
    pub fn build_of(&self, name: &str) -> String {
        fs::read_to_string(
            self.applications()
                .join(name)
                .join("Contents/MacOS/skillbinder"),
        )
        .unwrap()
    }
}

pub fn state(engine: &Engine) -> AppUpdateState {
    engine.current_app_update().unwrap().state
}

pub const UP_TO_DATE: AppUpdateState = AppUpdateState::Idle { last_failure: None };

pub fn ready(version: &str) -> AppUpdateState {
    AppUpdateState::Ready {
        version: version.parse().unwrap(),
        notes_url: format!("https://example.invalid/v{version}"),
    }
}
