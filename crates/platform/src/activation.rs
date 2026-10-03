//! A second process requests focus from the process holding the application lock.
//! This device-local marker avoids opening a network listener or accepting arbitrary messages.
use crate::paths::AppPaths;
use std::{fs::OpenOptions, io};

pub fn request(paths: &AppPaths) -> io::Result<()> {
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(paths.data.join("locks/activate"))?;
    Ok(())
}

pub fn take(paths: &AppPaths) -> io::Result<bool> {
    match std::fs::remove_file(paths.data.join("locks/activate")) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn activation_is_consumed_once_and_repeated_requests_coalesce() {
        let root =
            std::env::temp_dir().join(format!("skillbinder-activation-{}", uuid::Uuid::new_v4()));
        let paths = AppPaths::new(root.clone(), root.clone(), root.clone());
        paths.create_base_directories().unwrap();
        assert!(!take(&paths).unwrap());
        request(&paths).unwrap();
        request(&paths).unwrap();
        assert!(take(&paths).unwrap());
        assert!(!take(&paths).unwrap());
        std::fs::remove_dir_all(root).unwrap();
    }
}
