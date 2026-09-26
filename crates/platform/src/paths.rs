use std::path::{Path, PathBuf};

const ROOT_DIRECTORY: &str = ".skillbinder";

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data: PathBuf,
    pub config: PathBuf,
    pub cache: PathBuf,
}

impl AppPaths {
    pub fn new(data: PathBuf, config: PathBuf, cache: PathBuf) -> Self {
        Self {
            data,
            config,
            cache,
        }
    }

    /// One root for durable state, configuration, and disposable caches, so the whole application
    /// directory is discoverable and movable as a unit.
    pub fn for_home(home: &Path) -> Self {
        let root = home.join(ROOT_DIRECTORY);
        Self::new(root.clone(), root.clone(), root)
    }

    pub fn database(&self) -> PathBuf {
        self.data.join("state.sqlite")
    }

    pub fn library(&self) -> PathBuf {
        self.data.join("library")
    }

    pub fn logs(&self) -> PathBuf {
        self.data.join("logs")
    }

    pub fn git_sync_config(&self) -> PathBuf {
        self.config.join("git-sync.json")
    }

    pub fn lock(&self) -> PathBuf {
        self.data.join("locks").join("app.lock")
    }

    pub fn create_base_directories(&self) -> std::io::Result<()> {
        for path in [
            self.data.as_path(),
            self.config.as_path(),
            self.cache.as_path(),
            &self.data.join("locks"),
            &self.data.join("drafts"),
            &self.data.join("journals"),
            &self.data.join("staging"),
            &self.data.join("backups"),
            &self.data.join("recovery"),
            &self.data.join("logs"),
            &self.cache.join("source-repositories"),
            &self.cache.join("source-previews"),
        ] {
            std::fs::create_dir_all(path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_app_directory_is_one_root_under_the_home_directory() {
        let paths = AppPaths::for_home(Path::new("/home/dev"));
        assert_eq!(paths.data, PathBuf::from("/home/dev/.skillbinder"));
        assert_eq!(paths.config, paths.data);
        assert_eq!(paths.cache, paths.data);
        assert_eq!(
            paths.library(),
            PathBuf::from("/home/dev/.skillbinder/library")
        );
        assert_eq!(
            paths.database(),
            PathBuf::from("/home/dev/.skillbinder/state.sqlite")
        );
        assert_eq!(
            paths.git_sync_config(),
            PathBuf::from("/home/dev/.skillbinder/git-sync.json")
        );
    }
}
