//! A line's text: `<time> <LEVEL> <area> <message>[ <key>=<value>]…` (spec rules 1–5).

use super::filter::{Area, Level};
use regex::Regex;
use std::{
    fmt::{self, Write},
    path::Path,
    time::{Duration, SystemTime},
};

/// The longest line written, in characters.
const LINE_LIMIT: usize = 16_384;

/// One line, captured where it is logged and formatted on the writer thread.
pub(super) struct Record {
    pub at: SystemTime,
    pub level: Level,
    pub area: Area,
    pub message: &'static str,
    pub fields: Vec<(&'static str, String)>,
}

impl Record {
    /// The line, its fields redacted and quoted where needed, without a line break.
    pub(super) fn line(&self, redactor: &Redactor) -> String {
        let time =
            chrono::DateTime::<chrono::Local>::from(self.at).format("%Y-%m-%dT%H:%M:%S%.3f%:z");
        let mut line = format!(
            "{time} {:<5} {:<width$} {}",
            self.level,
            self.area.name(),
            self.message,
            width = Area::WIDTH
        );
        for (name, value) in &self.fields {
            let _ = write!(line, " {name}={}", Quoted(&redactor.redact(value)));
        }
        match line.char_indices().nth(LINE_LIMIT) {
            Some((end, _)) => line[..end].to_owned(),
            None => line,
        }
    }

    /// Whether this line repeats an earlier one: the same level, area, message, and fields.
    pub(super) fn key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (self.level, self.area, self.message, &self.fields).hash(&mut hasher);
        hasher.finish()
    }
}

/// A value, bare when it holds only safe characters and double-quoted with escapes otherwise, so
/// every line stays one line.
struct Quoted<'a>(&'a str);

impl fmt::Display for Quoted<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bare =
            |character: char| character.is_ascii_alphanumeric() || "._/:~@+-,".contains(character);
        if !self.0.is_empty() && self.0.chars().all(bare) {
            return formatter.write_str(self.0);
        }
        formatter.write_char('"')?;
        for character in self.0.chars() {
            match character {
                '"' => formatter.write_str("\\\"")?,
                '\\' => formatter.write_str("\\\\")?,
                '\n' => formatter.write_str("\\n")?,
                '\r' => formatter.write_str("\\r")?,
                '\t' => formatter.write_str("\\t")?,
                other => formatter.write_char(other)?,
            }
        }
        formatter.write_char('"')
    }
}

/// A duration as `840ms`, `2.9s`, `3m12s`, or `3h37m`.
pub(crate) struct Took(pub Duration);

impl fmt::Display for Took {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let seconds = self.0.as_secs();
        match seconds {
            0 => write!(formatter, "{}ms", self.0.as_millis()),
            1..60 => write!(formatter, "{:.1}s", self.0.as_secs_f64()),
            60..3600 => write!(formatter, "{}m{}s", seconds / 60, seconds % 60),
            _ => write!(formatter, "{}h{}m", seconds / 3600, seconds % 3600 / 60),
        }
    }
}

/// Keeps the home folder and credentials out of the log: the home folder becomes `~`, and URL
/// credentials, token-like query values, `Authorization` values, and bearer tokens become
/// `[redacted]`.
pub(super) struct Redactor {
    home: String,
    patterns: Vec<(Regex, &'static str)>,
}

impl Redactor {
    pub(super) fn new(home: &Path) -> Self {
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

    pub(super) fn redact(&self, text: &str) -> String {
        let mut result = if self.home.is_empty() {
            text.to_owned()
        } else {
            text.replace(&self.home, "~")
        };
        for (pattern, replacement) in &self.patterns {
            result = pattern.replace_all(&result, *replacement).into_owned();
        }
        result
    }
}
