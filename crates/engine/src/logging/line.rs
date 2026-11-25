//! A line's text: `<time> <LEVEL> <area> <message>[ <key>=<value>]…` (spec rules 1–5).

use super::{
    config::Sinks,
    filter::{Area, Level},
};
use regex::Regex;
use std::{
    fmt::{self, Write},
    path::Path,
    time::{Duration, SystemTime},
};

/// The longest line written, in characters.
const LINE_LIMIT: usize = 16_384;
/// A file line's time, such as `2026-11-25T14:02:15.911+01:00`, in characters.
const TIME_WIDTH: usize = 29;

/// How much of the time a line starts with.
#[derive(Clone, Copy)]
pub(super) enum Clock {
    /// Date, time, and offset: the file's lines.
    Date,
    /// The time alone: the console's lines (spec rule 23).
    Time,
}

/// One line, captured where it is logged and formatted on the writer thread.
pub(super) struct Record {
    pub at: SystemTime,
    pub level: Level,
    pub area: Area,
    pub message: &'static str,
    pub fields: Vec<(&'static str, String)>,
    pub sinks: Sinks,
}

impl Record {
    /// The time the line starts with.
    pub(super) fn time(&self, clock: Clock) -> impl fmt::Display {
        let time = chrono::DateTime::<chrono::Local>::from(self.at);
        time.format(match clock {
            Clock::Date => "%Y-%m-%dT%H:%M:%S%.3f%:z",
            Clock::Time => "%H:%M:%S%.3f",
        })
    }

    /// The line after its time, its fields redacted and quoted where needed, without a line
    /// break. With the file's time it is at most the line limit.
    pub(super) fn body(&self, redactor: &Redactor) -> String {
        let mut line = format!(
            "{:<5} {:<width$} {}",
            self.level,
            self.area.name(),
            self.message,
            width = Area::WIDTH
        );
        // Each value is cut to what is left of the limit before it is quoted, so a cut value
        // keeps its closing quote; fields that no longer fit are left out.
        let mut length = TIME_WIDTH + 1 + line.chars().count();
        for (name, value) in &self.fields {
            let Some(budget) = LINE_LIMIT
                .checked_sub(length + name.len() + 2)
                .filter(|budget| *budget >= 2)
            else {
                break;
            };
            let value = quote(&redactor.redact(value), budget);
            length += name.len() + 2 + value.chars().count();
            let _ = write!(line, " {name}={value}");
        }
        line
    }

    /// Whether this line repeats an earlier one: the same level, area, message, and fields.
    pub(super) fn key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (self.level, self.area, self.message, &self.fields).hash(&mut hasher);
        hasher.finish()
    }
}

/// `value`, bare when it holds only safe characters and double-quoted with escapes otherwise, so
/// every line stays one line, cut to at most `budget` characters, quotes included.
fn quote(value: &str, budget: usize) -> String {
    let bare =
        |character: char| character.is_ascii_alphanumeric() || "._/:~@+-,".contains(character);
    if !value.is_empty() && value.chars().all(bare) {
        return value.chars().take(budget).collect();
    }
    let mut quoted = String::from('"');
    let mut used = 2;
    for character in value.chars() {
        let escape = match character {
            '"' => Some('"'),
            '\\' => Some('\\'),
            '\n' => Some('n'),
            '\r' => Some('r'),
            '\t' => Some('t'),
            _ => None,
        };
        used += if escape.is_some() { 2 } else { 1 };
        if used > budget {
            break;
        }
        match escape {
            Some(escape) => {
                quoted.push('\\');
                quoted.push(escape);
            }
            None => quoted.push(character),
        }
    }
    quoted.push('"');
    quoted
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
