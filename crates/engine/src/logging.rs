use regex::Regex;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc::{self, SyncSender},
    time::{Duration, SystemTime},
};

#[derive(Clone)]
pub(crate) struct Logger {
    sender: SyncSender<Message>,
}
enum Message {
    Line(String),
    Flush(mpsc::Sender<()>),
}
impl Logger {
    pub(crate) fn start(directory: PathBuf, home: &Path) -> std::io::Result<Self> {
        fs::create_dir_all(&directory)?;
        let redactor = Redactor::new(home);
        let (sender, receiver) = mpsc::sync_channel(256);
        std::thread::Builder::new()
            .name("skillbinder-logs".into())
            .spawn(move || {
                for message in receiver {
                    match message {
                        Message::Line(line) => {
                            let _ = append(&directory, &redactor.redact(&line));
                        }
                        Message::Flush(done) => {
                            let _ = done.send(());
                        }
                    }
                }
            })?;
        Ok(Self { sender })
    }
    pub(crate) fn record(&self, line: impl Into<String>) {
        let _ = self.sender.try_send(Message::Line(line.into()));
    }
    pub(crate) fn flush(&self) {
        let (sender, receiver) = mpsc::channel();
        if self.sender.send(Message::Flush(sender)).is_ok() {
            let _ = receiver.recv();
        }
    }
}
struct Redactor {
    home: String,
    patterns: Vec<(Regex, &'static str)>,
}
impl Redactor {
    fn new(home: &Path) -> Self {
        let patterns = [
            (r"(?i)(https?://)[^\s/@]+(?::[^\s/@]*)?@", "$1[redacted]@"),
            (r"(?i)([?&](?:token|key|api_key|apikey|password|secret|access_token|auth|credential)=)[^&\s]+", "$1[redacted]"),
            (r"(?i)(authorization\s*[:=]\s*)[^\r\n]+", "$1[redacted]"),
            (r"(?i)(bearer\s+)[A-Za-z0-9._~+/=-]+", "$1[redacted]"),
        ].into_iter().filter_map(|(pattern, replacement)| Regex::new(pattern).ok().map(|regex| (regex, replacement))).collect();
        Self {
            home: home.to_string_lossy().into_owned(),
            patterns,
        }
    }
    fn redact(&self, line: &str) -> String {
        let bounded: String = line.chars().take(16_384).collect();
        let mut result = if self.home.is_empty() {
            bounded
        } else {
            bounded.replace(&self.home, "~")
        };
        for (pattern, replacement) in &self.patterns {
            result = pattern.replace_all(&result, *replacement).into_owned();
        }
        result
    }
}
fn append(directory: &Path, line: &str) -> std::io::Result<()> {
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
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    writeln!(file, "{timestamp} {}", line.replace(['\n', '\r'], " "))?;
    file.flush()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn credentials_and_home_never_reach_logs() {
        let output = Redactor::new(Path::new("/private/home")).redact("/private/home/a https://user:secret@example.test/a?token=hidden&x=ok Bearer abc123\nAuthorization: Basic secret");
        assert!(
            ![
                "/private/home",
                "user:secret",
                "hidden",
                "abc123",
                "Basic secret"
            ]
            .iter()
            .any(|secret| output.contains(secret)),
            "{output}"
        );
        assert!(output.contains("~/a"));
    }
}
