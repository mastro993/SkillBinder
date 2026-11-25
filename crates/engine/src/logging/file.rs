//! `skillbinder.log`: 5 MiB per file, five files, kept 14 days.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    time::Duration,
};

/// Appends `line`, first removing old files and rotating a full one.
pub(super) fn append(directory: &Path, line: &str) -> std::io::Result<()> {
    let path = directory.join("skillbinder.log");
    for index in 1..5 {
        let old = directory.join(format!("skillbinder.{index}.log"));
        if fs::metadata(&old)
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|time| time.elapsed().ok())
            .is_some_and(|age| age > Duration::from_secs(14 * 86400))
        {
            let _ = fs::remove_file(old);
        }
    }
    if fs::metadata(&path)
        .is_ok_and(|metadata| metadata.len() + line.len() as u64 > 5 * 1024 * 1024)
    {
        let _ = fs::remove_file(directory.join("skillbinder.4.log"));
        for index in (1..4).rev() {
            let _ = fs::rename(
                directory.join(format!("skillbinder.{index}.log")),
                directory.join(format!("skillbinder.{}.log", index + 1)),
            );
        }
        fs::rename(&path, directory.join("skillbinder.1.log"))?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")?;
    file.flush()
}
