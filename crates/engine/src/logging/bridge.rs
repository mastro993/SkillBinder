//! The `log` facade's records as lines: the desktop's own as `app`, GPUI's and Kit's as `ui`
//! (spec rule 21).

use super::{Area, Level, Logger};

/// Writes `log` records to a [`Logger`] under the fixed message `record`, with the record's
/// target in `target` and its free text in `text`. The desktop installs it as the process's
/// logger.
pub struct Bridge {
    log: Logger,
}

impl Bridge {
    /// A bridge writing to `log`.
    pub fn new(log: Logger) -> Self {
        Self { log }
    }

    /// The most detailed level either sink writes for bridged records, for
    /// `log::set_max_level`, so a record neither writes is never formatted.
    pub fn max_level(&self) -> log::LevelFilter {
        let config = &self.log.shared.config;
        match config
            .most_detailed(Area::App)
            .max(config.most_detailed(Area::Ui))
        {
            None => log::LevelFilter::Off,
            Some(Level::Error) => log::LevelFilter::Error,
            Some(Level::Warn) => log::LevelFilter::Warn,
            Some(Level::Info) => log::LevelFilter::Info,
            Some(Level::Debug) => log::LevelFilter::Debug,
        }
    }
}

/// SkillBinder's own targets, the desktop's among them, are `app`; every other is `ui`.
fn area(target: &str) -> Area {
    if target.starts_with("skillbinder") {
        Area::App
    } else {
        Area::Ui
    }
}

fn level(level: log::Level) -> Level {
    match level {
        log::Level::Error => Level::Error,
        log::Level::Warn => Level::Warn,
        log::Level::Info => Level::Info,
        log::Level::Debug | log::Level::Trace => Level::Debug,
    }
}

impl log::Log for Bridge {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        let config = &self.log.shared.config;
        config
            .sinks(level(metadata.level()), area(metadata.target()))
            .any()
    }

    fn log(&self, record: &log::Record<'_>) {
        let target = record.target();
        self.log
            .event(level(record.level()), area(target), "record")
            .field("target", target)
            .field("text", record.args());
    }

    fn flush(&self) {
        self.log.flush();
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::logging::{Console, LogConfig, LogFilter};
    use log::Log;

    fn bridge(config: LogConfig) -> (tempfile::TempDir, Bridge) {
        let root = tempfile::tempdir().unwrap();
        let log = Logger::start(root.path().join("logs"), root.path(), config).unwrap();
        (root, Bridge::new(log))
    }

    fn emit(bridge: &Bridge, level: log::Level, target: &str, text: &str) {
        bridge.log(
            &log::Record::builder()
                .level(level)
                .target(target)
                .args(format_args!("{text}"))
                .build(),
        );
    }

    #[test]
    fn records_become_app_and_ui_lines() {
        let (root, bridge) = bridge(LogConfig::default());
        emit(
            &bridge,
            log::Level::Warn,
            "gpui::text_system",
            "Failed to load font \"Optima\"",
        );
        emit(&bridge, log::Level::Info, "gpui::window", "frame");
        emit(
            &bridge,
            log::Level::Error,
            "skillbinder::command_server",
            "The skillbinder command can't reach this app: denied",
        );
        bridge.flush();
        let text = std::fs::read_to_string(root.path().join("logs/skillbinder.log")).unwrap();
        let lines: Vec<_> = text.lines().collect();
        let shape = regex::Regex::new(crate::logging::tests::LINE).unwrap();
        assert_eq!(lines.len(), 2, "ui stays at warn: {lines:?}");
        assert!(
            lines[0].ends_with(
                r#" WARN  ui           record target=gpui::text_system text="Failed to load font \"Optima\"""#
            ),
            "{}",
            lines[0]
        );
        assert!(
            lines[1].contains(" ERROR app          record target=skillbinder::command_server "),
            "{}",
            lines[1]
        );
        for line in lines {
            assert!(shape.is_match(line), "{line}");
        }
    }

    #[test]
    fn the_facade_asks_only_for_what_a_sink_writes() {
        let (_root, quiet) = bridge(LogConfig::default());
        assert_eq!(quiet.max_level(), log::LevelFilter::Info);
        let ui_info = log::Metadata::builder()
            .level(log::Level::Info)
            .target("gpui")
            .build();
        assert!(!quiet.enabled(&ui_info));

        let (_root, development) = bridge(LogConfig::new(None, Console::Development));
        assert_eq!(development.max_level(), log::LevelFilter::Debug);
        assert!(!development.enabled(&ui_info));

        let (_root, off) = bridge(LogConfig {
            file: LogFilter::parse("off"),
            console: None,
        });
        assert_eq!(off.max_level(), log::LevelFilter::Off);
    }
}
