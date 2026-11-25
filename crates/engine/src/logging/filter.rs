//! Levels, areas, and the `SKILLBINDER_LOG` filter that chooses which lines are written.

use std::fmt;

/// How bad a line is. Ordered from the most to the least severe, so a filter passes every level
/// up to its own.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) enum Level {
    /// An action the user started failed, or data needs recovery.
    Error,
    /// Something degraded that the app works around or retries.
    Warn,
    /// The trail: sessions, actions, and background jobs' summaries.
    Info,
    /// Per-item detail.
    Debug,
}

impl Level {
    fn parse(text: &str) -> Option<Option<Self>> {
        Some(Some(match text {
            "error" => Self::Error,
            "warn" => Self::Warn,
            "info" => Self::Info,
            "debug" => Self::Debug,
            "off" => return Some(None),
            _ => return None,
        }))
    }
}

impl fmt::Display for Level {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.pad(match self {
            Self::Error => "ERROR",
            Self::Warn => "WARN",
            Self::Info => "INFO",
            Self::Debug => "DEBUG",
        })
    }
}

/// Where a line comes from (spec rule 9).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum Area {
    Session,
    /// The desktop process: window, command socket, launch.
    App,
    /// GPUI and Kit, through the `log` facade.
    Ui,
    Library,
    Scan,
    Import,
    Install,
    /// Skill updates from sources.
    Update,
    Sync,
    /// Activation and managed copies.
    Copies,
    Presets,
    Workspaces,
    Agents,
    Explore,
    AppUpdate,
    CommandLine,
    Mcp,
    /// The database, journals, and recovery.
    Storage,
}

impl Area {
    const ALL: [Self; 18] = [
        Self::Session,
        Self::App,
        Self::Ui,
        Self::Library,
        Self::Scan,
        Self::Import,
        Self::Install,
        Self::Update,
        Self::Sync,
        Self::Copies,
        Self::Presets,
        Self::Workspaces,
        Self::Agents,
        Self::Explore,
        Self::AppUpdate,
        Self::CommandLine,
        Self::Mcp,
        Self::Storage,
    ];
    /// The longest name's length, which every area is padded to.
    pub(super) const WIDTH: usize = 12;

    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Session => "session",
            Self::App => "app",
            Self::Ui => "ui",
            Self::Library => "library",
            Self::Scan => "scan",
            Self::Import => "import",
            Self::Install => "install",
            Self::Update => "update",
            Self::Sync => "sync",
            Self::Copies => "copies",
            Self::Presets => "presets",
            Self::Workspaces => "workspaces",
            Self::Agents => "agents",
            Self::Explore => "explore",
            Self::AppUpdate => "app-update",
            Self::CommandLine => "command-line",
            Self::Mcp => "mcp",
            Self::Storage => "storage",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|area| area.name() == text)
    }
}

/// Which lines are written: a level for every area, `None` writing none of its lines. Parsed from
/// `SKILLBINDER_LOG` (spec rule 8), whose directives a launch's first lines repeat.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogFilter {
    levels: [Option<Level>; Area::ALL.len()],
    /// `SKILLBINDER_LOG` as set, if it was.
    setting: Option<String>,
    /// The directives naming an unknown area or level, which are ignored.
    ignored: Vec<String>,
}

impl Default for LogFilter {
    /// `INFO` and above, and `WARN` and above for `ui` (spec rule 7).
    fn default() -> Self {
        let mut levels = [Some(Level::Info); Area::ALL.len()];
        levels[Area::Ui as usize] = Some(Level::Warn);
        Self {
            levels,
            setting: None,
            ignored: Vec::new(),
        }
    }
}

impl LogFilter {
    /// Reads comma-separated directives: a level alone sets every area's, replacing the default
    /// one of `ui`, and `area=level` sets one area's. Levels are `error`, `warn`, `info`,
    /// `debug`, and `off`. A directive naming an unknown area or level is ignored and kept, so
    /// the session can say so.
    pub fn parse(setting: &str) -> Self {
        let mut filter = Self {
            setting: Some(setting.to_owned()),
            ..Self::default()
        };
        let mut areas = Vec::new();
        for directive in setting.split(',').map(str::trim) {
            let lowered = directive.to_ascii_lowercase();
            let parsed = match lowered.split_once('=') {
                Some((area, level)) => Area::parse(area.trim())
                    .zip(Level::parse(level.trim()))
                    .map(|(area, level)| areas.push((area, level))),
                None if lowered.is_empty() => Some(()),
                None => {
                    Level::parse(&lowered).map(|level| filter.levels = [level; Area::ALL.len()])
                }
            };
            if parsed.is_none() {
                filter.ignored.push(directive.to_owned());
            }
        }
        for (area, level) in areas {
            filter.levels[area as usize] = level;
        }
        filter
    }

    /// Whether a line of `level` in `area` is written.
    pub(crate) fn passes(&self, level: Level, area: Area) -> bool {
        self.levels[area as usize].is_some_and(|threshold| level <= threshold)
    }

    /// `SKILLBINDER_LOG` as set, if it was.
    pub(super) fn setting(&self) -> Option<&str> {
        self.setting.as_deref()
    }

    /// The directives that were ignored.
    pub(super) fn ignored(&self) -> &[String] {
        &self.ignored
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_writes_info_and_ui_warnings() {
        let filter = LogFilter::default();
        assert!(filter.passes(Level::Info, Area::Scan));
        assert!(!filter.passes(Level::Debug, Area::Scan));
        assert!(filter.passes(Level::Warn, Area::Ui));
        assert!(!filter.passes(Level::Info, Area::Ui));
        assert_eq!(filter.setting(), None);
    }

    #[test]
    fn directives_set_the_default_and_single_areas() {
        let filter = LogFilter::parse("info, scan=debug,update=DEBUG");
        assert!(filter.passes(Level::Debug, Area::Scan));
        assert!(filter.passes(Level::Debug, Area::Update));
        assert!(!filter.passes(Level::Debug, Area::Install));
        assert!(
            filter.passes(Level::Info, Area::Ui),
            "a level alone sets ui too"
        );
        assert!(filter.ignored().is_empty());

        let quiet = LogFilter::parse("off,session=info");
        assert!(!quiet.passes(Level::Error, Area::Install));
        assert!(quiet.passes(Level::Info, Area::Session));

        let area_only = LogFilter::parse("scan=debug");
        assert!(area_only.passes(Level::Info, Area::Install));
        assert!(
            !area_only.passes(Level::Info, Area::Ui),
            "ui keeps its default"
        );
    }

    #[test]
    fn unknown_areas_and_levels_are_ignored_and_kept() {
        let filter = LogFilter::parse("bogus,scan=loud,nowhere=debug,warn");
        assert_eq!(filter.ignored(), ["bogus", "scan=loud", "nowhere=debug"]);
        assert!(!filter.passes(Level::Info, Area::Scan));
        assert_eq!(filter.setting(), Some("bogus,scan=loud,nowhere=debug,warn"));
    }
}
