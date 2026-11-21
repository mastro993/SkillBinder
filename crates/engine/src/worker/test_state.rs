//! The worker state a unit test opens on an isolated data directory, without a worker thread.

use super::{DATABASE, State};
use crate::{
    activation::Progress,
    app_update::{AppUpdates, running_version},
    catalog::Catalogs,
    database,
    git::Installer,
    lifecycle::EngineConfig,
    logging::Logger,
    scan::Scans,
    shutdown::Shutdown,
    sources::Updates,
};
use std::{cell::RefCell, collections::BTreeMap, fs, fs::File, path::PathBuf};

#[expect(
    clippy::unwrap_used,
    reason = "Test fixture failures must abort the test."
)]
impl State {
    pub(crate) fn isolated_test_state(data_dir: PathBuf) -> Self {
        let home = data_dir.join("home");
        for path in [
            home.clone(),
            data_dir.join("staging"),
            data_dir.join("library/skills"),
            data_dir.join("logs"),
        ] {
            fs::create_dir_all(path).unwrap();
        }
        let log = Logger::start(data_dir.join("logs"), &home).unwrap();
        let config = EngineConfig::isolated(data_dir.clone(), home.clone());
        Self {
            app_updates: AppUpdates::new(
                None,
                running_version(),
                config.placement.clone(),
                data_dir.join("updates"),
                log.clone(),
            ),
            config,
            database: RefCell::new(database::open(&data_dir.join(DATABASE)).unwrap()),
            git: None,
            outdated_git: None,
            installer: Installer::detect(),
            log,
            scans: Scans::default(),
            updates: Updates::default(),
            catalog: Catalogs::open(
                crate::CatalogSource::Directory(data_dir.join("catalog-source")),
                &data_dir,
                Shutdown::default(),
            ),
            installs: Default::default(),
            previews: Default::default(),
            repositories: Default::default(),
            searches: crate::catalog::Searches::new(crate::CatalogSource::Directory(
                data_dir.join("catalog-source"),
            )),
            resolutions: Default::default(),
            progress: Progress::default(),
            grants: BTreeMap::new(),
            recovery_required: false,
            shutdown: Shutdown::default(),
            lock: File::create(data_dir.join("test.lock")).unwrap(),
        }
    }
}
