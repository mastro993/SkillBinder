use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub(crate) const ACTIVE_FILE_NAME: &str = "skillbinder.log";

#[derive(Debug, Clone, Copy)]
pub struct RotationPolicy {
    pub max_file_bytes: u64,
    pub max_files: usize,
    pub max_age: Duration,
}

impl Default for RotationPolicy {
    fn default() -> Self {
        Self {
            max_file_bytes: 5 * 1024 * 1024,
            max_files: 5,
            max_age: Duration::from_secs(14 * 24 * 60 * 60),
        }
    }
}

pub enum ActiveFile {
    Open {
        file: File,
        len: u64,
        opened_at: SystemTime,
    },
    Missing,
}

pub struct RotatingLog {
    dir: PathBuf,
    policy: RotationPolicy,
    active: ActiveFile,
    degraded: bool,
    now: fn() -> SystemTime,
}

impl RotatingLog {
    pub fn open(dir: impl Into<PathBuf>, policy: RotationPolicy) -> io::Result<Self> {
        Self::open_with_clock(dir, policy, SystemTime::now)
    }

    pub fn open_with_clock(
        dir: impl Into<PathBuf>,
        policy: RotationPolicy,
        now: fn() -> SystemTime,
    ) -> io::Result<Self> {
        let dir = dir.into();
        fs::create_dir_all(&dir)?;
        let active_path = dir.join(ACTIVE_FILE_NAME);
        repair_torn_line(&active_path)?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&active_path)?;
        let metadata = file.metadata()?;
        let len = metadata.len();
        let opened_at = if len == 0 {
            now()
        } else {
            metadata.modified().unwrap_or_else(|_| now())
        };
        let log = Self {
            dir,
            policy,
            active: ActiveFile::Open {
                file,
                len,
                opened_at,
            },
            degraded: false,
            now,
        };
        log.prune_aged_backups();
        Ok(log)
    }

    pub fn write_line(&mut self, line: &[u8]) {
        if self.degraded {
            return;
        }
        if self.should_rotate(line.len())
            && let Err(error) = self.rotate()
        {
            self.degrade(&error);
            return;
        }
        if let Err(error) = self.append(line) {
            self.degrade(&error);
        }
    }

    fn should_rotate(&self, line_len: usize) -> bool {
        match &self.active {
            ActiveFile::Open { len, opened_at, .. } => {
                let written = len.saturating_add(line_len as u64).saturating_add(1);
                written > self.policy.max_file_bytes
                    || (self.now)()
                        .duration_since(*opened_at)
                        .is_ok_and(|age| age >= self.policy.max_age)
            }
            ActiveFile::Missing => false,
        }
    }

    fn append(&mut self, line: &[u8]) -> io::Result<()> {
        if let ActiveFile::Open { file, len, .. } = &mut self.active {
            file.write_all(line)?;
            file.write_all(b"\n")?;
            *len += line.len() as u64 + 1;
        }
        Ok(())
    }

    fn rotate(&mut self) -> io::Result<()> {
        let current = std::mem::replace(&mut self.active, ActiveFile::Missing);
        if let ActiveFile::Open { file, .. } = current {
            drop(file);
        }
        let active_path = self.active_path();
        let backup_cap = self.backup_cap();
        if backup_cap == 0 {
            remove_if_exists(&active_path)?;
        } else {
            remove_if_exists(&self.backup_path(backup_cap))?;
            for index in (1..backup_cap).rev() {
                rename_if_exists(&self.backup_path(index), &self.backup_path(index + 1))?;
            }
            rename_if_exists(&active_path, &self.backup_path(1))?;
        }
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&active_path)?;
        self.active = ActiveFile::Open {
            file,
            len: 0,
            opened_at: (self.now)(),
        };
        self.prune_aged_backups();
        Ok(())
    }

    fn prune_aged_backups(&self) {
        for index in 1..=self.backup_cap() {
            let path = self.backup_path(index);
            let modified = match fs::metadata(&path).and_then(|metadata| metadata.modified()) {
                Ok(modified) => modified,
                Err(_) => continue,
            };
            let expired = (self.now)()
                .duration_since(modified)
                .is_ok_and(|age| age > self.policy.max_age);
            if expired {
                let _ = fs::remove_file(path);
            }
        }
    }

    fn degrade(&mut self, error: &io::Error) {
        self.degraded = true;
        self.active = ActiveFile::Missing;
        eprintln!("skillbinder: log file unusable, every later line is dropped: {error}");
    }

    fn active_path(&self) -> PathBuf {
        self.dir.join(ACTIVE_FILE_NAME)
    }

    fn backup_cap(&self) -> usize {
        self.policy.max_files.saturating_sub(1)
    }

    fn backup_path(&self, index: usize) -> PathBuf {
        self.dir.join(format!("{ACTIVE_FILE_NAME}.{index}"))
    }
}

fn repair_torn_line(path: &Path) -> io::Result<()> {
    let mut file = match OpenOptions::new().read(true).write(true).open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if file.metadata()?.len() == 0 {
        return Ok(());
    }
    file.seek(SeekFrom::End(-1))?;
    let mut last = [0u8; 1];
    file.read_exact(&mut last)?;
    if last[0] == b'\n' {
        return Ok(());
    }
    file.seek(SeekFrom::End(0))?;
    file.write_all(b"\n")
}

fn remove_if_exists(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn rename_if_exists(from: &Path, to: &Path) -> io::Result<()> {
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static AGE_CLOCK_SECONDS: AtomicU64 = AtomicU64::new(1_700_000_000);
    static FUTURE_CLOCK_SECONDS: AtomicU64 = AtomicU64::new(32_500_000_000);

    fn age_clock() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(AGE_CLOCK_SECONDS.load(Ordering::Relaxed))
    }

    fn future_clock() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(FUTURE_CLOCK_SECONDS.load(Ordering::Relaxed))
    }

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("skillbinder-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("create temp dir");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn roomy() -> RotationPolicy {
        RotationPolicy {
            max_file_bytes: 4096,
            max_files: 5,
            max_age: Duration::from_secs(3600),
        }
    }

    fn read(path: &Path) -> String {
        String::from_utf8(fs::read(path).expect("read log file")).expect("utf8 log file")
    }

    fn log_file_count(dir: &Path) -> usize {
        fs::read_dir(dir)
            .expect("read log dir")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(ACTIVE_FILE_NAME)
            })
            .count()
    }

    #[test]
    fn size_boundary_creates_the_first_backup() {
        let dir = TempDir::new("rotating-size");
        let policy = RotationPolicy {
            max_file_bytes: 32,
            ..roomy()
        };
        let mut log = RotatingLog::open(dir.path(), policy).expect("open");
        log.write_line(b"line-0000000000");
        log.write_line(b"line-0000000001");
        log.write_line(b"line-0000000002");

        assert_eq!(
            read(&dir.path().join("skillbinder.log.1")),
            "line-0000000000\nline-0000000001\n"
        );
        assert_eq!(
            read(&dir.path().join("skillbinder.log")),
            "line-0000000002\n"
        );
    }

    #[test]
    fn backup_count_never_exceeds_the_policy() {
        let dir = TempDir::new("rotating-cap");
        let policy = RotationPolicy {
            max_file_bytes: 16,
            max_files: 3,
            ..roomy()
        };
        let mut log = RotatingLog::open(dir.path(), policy).expect("open");
        for index in 0..20 {
            log.write_line(format!("line-{index}").as_bytes());
        }

        assert!(log_file_count(dir.path()) <= policy.max_files);
        assert_eq!(log_file_count(dir.path()), policy.max_files);
    }

    #[test]
    fn an_aged_active_file_rotates_on_the_next_write() {
        let dir = TempDir::new("rotating-age");
        let policy = RotationPolicy {
            max_age: Duration::from_secs(3600),
            ..roomy()
        };
        let mut log = RotatingLog::open_with_clock(dir.path(), policy, age_clock).expect("open");
        log.write_line(b"before the window");
        AGE_CLOCK_SECONDS.fetch_add(policy.max_age.as_secs() + 1, Ordering::Relaxed);
        log.write_line(b"after the window");

        assert_eq!(
            read(&dir.path().join("skillbinder.log.1")),
            "before the window\n"
        );
        assert_eq!(
            read(&dir.path().join("skillbinder.log")),
            "after the window\n"
        );
    }

    #[test]
    fn a_torn_last_line_gains_a_newline_on_reopen() {
        let dir = TempDir::new("rotating-torn");
        let path = dir.path().join("skillbinder.log");
        fs::write(&path, b"truncated line without newline").expect("seed torn line");
        let mut log = RotatingLog::open(dir.path(), roomy()).expect("open");
        log.write_line(b"next line");

        assert_eq!(read(&path), "truncated line without newline\nnext line\n");
    }

    #[test]
    fn an_active_file_older_than_the_policy_rotates_after_a_restart() {
        let dir = TempDir::new("rotating-restart");
        let policy = RotationPolicy {
            max_age: Duration::from_secs(3600),
            ..roomy()
        };
        fs::write(dir.path().join("skillbinder.log"), b"stale line\n").expect("seed active file");
        let mut log = RotatingLog::open_with_clock(dir.path(), policy, future_clock).expect("open");
        log.write_line(b"new line");

        assert_eq!(read(&dir.path().join("skillbinder.log")), "new line\n");
    }

    #[test]
    fn opening_a_log_directory_prunes_an_aged_backup_without_a_write() {
        let dir = TempDir::new("rotating-open-prune");
        let policy = RotationPolicy {
            max_age: Duration::from_secs(3600),
            ..roomy()
        };
        fs::write(dir.path().join("skillbinder.log"), b"active line\n").expect("seed active file");
        fs::write(dir.path().join("skillbinder.log.1"), b"aged backup\n").expect("seed backup");
        let _log = RotatingLog::open_with_clock(dir.path(), policy, future_clock).expect("open");

        assert!(!dir.path().join("skillbinder.log.1").exists());
        assert_eq!(log_file_count(dir.path()), 1);
        assert_eq!(read(&dir.path().join("skillbinder.log")), "active line\n");
    }

    #[test]
    fn aged_backups_are_pruned_on_rotation() {
        let dir = TempDir::new("rotating-prune");
        let policy = RotationPolicy {
            max_file_bytes: 1,
            max_age: Duration::from_secs(3600),
            ..roomy()
        };
        fs::write(dir.path().join("skillbinder.log"), b"stale active\n").expect("seed active");
        fs::write(dir.path().join("skillbinder.log.1"), b"stale backup\n").expect("seed backup");
        let mut log = RotatingLog::open_with_clock(dir.path(), policy, future_clock).expect("open");
        log.write_line(b"fresh");

        assert_eq!(log_file_count(dir.path()), 1);
        assert_eq!(read(&dir.path().join("skillbinder.log")), "fresh\n");
    }

    #[test]
    fn a_failed_rotation_degrades_the_writer_instead_of_growing_the_file() {
        let dir = TempDir::new("rotating-failure");
        let policy = RotationPolicy {
            max_file_bytes: 1,
            max_files: 2,
            ..roomy()
        };
        let blocker = dir.path().join("skillbinder.log.1");
        fs::create_dir_all(&blocker).expect("create blocking directory");
        fs::write(blocker.join("keep"), b"x").expect("fill blocking directory");
        let mut log = RotatingLog::open(dir.path(), policy).expect("open");

        log.write_line(b"line one");
        assert!(log.degraded);

        let active = dir.path().join("skillbinder.log");
        let length = fs::metadata(&active).expect("stat active").len();
        log.write_line(b"line two");
        log.write_line(b"line three");
        assert_eq!(fs::metadata(&active).expect("stat active").len(), length);
        assert_eq!(length, 0);
    }
}
