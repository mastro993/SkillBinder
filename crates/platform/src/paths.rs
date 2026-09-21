use std::path::PathBuf;

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

    pub fn database(&self) -> PathBuf {
        self.data.join("state.sqlite")
    }

    pub fn library(&self) -> PathBuf {
        self.data.join("library")
    }

    pub fn logs(&self) -> PathBuf {
        self.data.join("logs")
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
