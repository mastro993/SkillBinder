//! The two sinks, the file and the console, each with its own filter (spec rules 7, 8, and 22).

use super::{Area, Level, LogFilter};

/// What the app's stderr is, which decides whether the console shows and at what level.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Console {
    /// A debug build: the console shows whether or not stderr is a terminal (spec D5).
    Development,
    /// A release build whose stderr is a terminal.
    Terminal,
    /// A release build whose stderr is not a terminal, such as one started from Finder: the
    /// console shows nothing.
    Hidden,
}

/// Which lines the file and the console write. The default writes the file at its default level
/// and hides the console.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LogConfig {
    pub(super) file: LogFilter,
    /// `None` when the console shows nothing.
    pub(super) console: Option<LogFilter>,
}

impl LogConfig {
    /// The sinks for `SKILLBINDER_LOG`'s `setting`, if it is set. A setting filters the file and
    /// the console alike; without one, the file writes its default and a debug build's console
    /// writes `debug,ui=warn`, while a release build's writes the file's default.
    pub fn new(setting: Option<&str>, console: Console) -> Self {
        let file = setting.map(LogFilter::parse).unwrap_or_default();
        let console = match (console, setting) {
            (Console::Hidden, _) => None,
            (_, Some(_)) => Some(file.clone()),
            (Console::Development, None) => Some(LogFilter::development()),
            (Console::Terminal, None) => Some(LogFilter::default()),
        };
        Self { file, console }
    }

    /// The sinks a line of `level` in `area` goes to.
    pub(super) fn sinks(&self, level: Level, area: Area) -> Sinks {
        Sinks {
            file: self.file.passes(level, area),
            console: self
                .console
                .as_ref()
                .is_some_and(|console| console.passes(level, area)),
        }
    }

    /// The most detailed level either sink writes in `area`, if any.
    pub(super) fn most_detailed(&self, area: Area) -> Option<Level> {
        let console = self
            .console
            .as_ref()
            .and_then(|console| console.level(area));
        self.file.level(area).max(console)
    }
}

/// Where one line goes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Sinks {
    pub file: bool,
    pub console: bool,
}

impl Sinks {
    pub(super) fn any(self) -> bool {
        self.file || self.console
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCAN_DEBUG: (Level, Area) = (Level::Debug, Area::Scan);
    const UI_INFO: (Level, Area) = (Level::Info, Area::Ui);

    fn sinks(config: &LogConfig, (level, area): (Level, Area)) -> Sinks {
        config.sinks(level, area)
    }

    #[test]
    fn a_debug_build_echoes_debug_while_the_file_keeps_info() {
        let config = LogConfig::new(None, Console::Development);
        let only_console = Sinks {
            file: false,
            console: true,
        };
        assert_eq!(sinks(&config, SCAN_DEBUG), only_console);
        assert!(!sinks(&config, UI_INFO).any(), "ui stays at warn");
        assert!(sinks(&config, (Level::Warn, Area::Ui)).file);
        assert_eq!(config.file.setting(), None);
    }

    #[test]
    fn a_setting_filters_both_sinks_in_every_build() {
        for console in [Console::Development, Console::Terminal] {
            let config = LogConfig::new(Some("warn"), console);
            assert_eq!(config.console.as_ref(), Some(&config.file));
            assert!(!sinks(&config, (Level::Info, Area::Scan)).any());
        }
        let hidden = LogConfig::new(Some("scan=debug"), Console::Hidden);
        assert_eq!(hidden.console, None);
        assert!(sinks(&hidden, SCAN_DEBUG).file);
    }

    #[test]
    fn a_release_build_echoes_the_files_default_only_to_a_terminal() {
        let terminal = LogConfig::new(None, Console::Terminal);
        assert_eq!(terminal.console, Some(LogFilter::default()));
        assert!(!sinks(&terminal, SCAN_DEBUG).any());
        assert_eq!(LogConfig::new(None, Console::Hidden), LogConfig::default());
    }

    #[test]
    fn the_most_detailed_level_spans_both_sinks() {
        let config = LogConfig::new(None, Console::Development);
        assert_eq!(config.most_detailed(Area::App), Some(Level::Debug));
        assert_eq!(config.most_detailed(Area::Ui), Some(Level::Warn));
        let off = LogConfig::new(Some("off"), Console::Hidden);
        assert_eq!(off.most_detailed(Area::App), None);
    }
}
