#![allow(clippy::unwrap_used)]
use super::*;
use regex::Regex;
use std::time::Duration;

/// Spec AC1: every line has this shape.
const LINE: &str = r#"^\S+ (ERROR|WARN |INFO |DEBUG) [a-z-]+ +[a-z][a-z ]*[a-z]( [a-z.]+=("([^"\\]|\\.)*"|\S+))*$"#;

fn logger(filter: LogFilter) -> (tempfile::TempDir, Logger) {
    let root = tempfile::tempdir().unwrap();
    let log = Logger::start(root.path().join("logs"), Path::new("/private/home"), filter).unwrap();
    (root, log)
}

fn lines(root: &tempfile::TempDir, log: &Logger) -> Vec<String> {
    log.flush();
    std::fs::read_to_string(root.path().join("logs/skillbinder.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn lines_have_a_time_level_area_message_and_fields() {
    let (root, log) = logger(LogFilter::default());
    log.event(Level::Info, Area::Scan, "finished")
        .field("locations", 412)
        .took(Instant::now());
    log.event(Level::Error, Area::CommandLine, "command change failed")
        .field("cause", "line one\nline \"two\"\t\\")
        .field("slugs", "a,b")
        .field("text", "");
    let lines = lines(&root, &log);
    let shape = Regex::new(LINE).unwrap();
    assert_eq!(lines.len(), 2, "{lines:?}");
    for line in &lines {
        assert!(shape.is_match(line), "{line}");
    }
    let time = Regex::new(r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{3}[+-]\d\d:\d\d ").unwrap();
    assert!(time.is_match(&lines[0]), "{}", lines[0]);
    assert!(lines[0].contains(" INFO  scan         finished locations=412 took="));
    assert!(lines[1].ends_with(
        r#"ERROR command-line command change failed cause="line one\nline \"two\"\t\\" slugs=a,b text="""#
    ));
}

#[test]
fn the_filter_decides_before_anything_is_written() {
    let (root, log) = logger(LogFilter::default());
    log.event(Level::Debug, Area::Scan, "location absent");
    log.event(Level::Info, Area::Ui, "record");
    log.event(Level::Warn, Area::Ui, "record");
    assert_eq!(lines(&root, &log).len(), 1);

    let (root, log) = logger(LogFilter::parse("scan=debug"));
    log.event(Level::Debug, Area::Scan, "location absent");
    log.event(Level::Debug, Area::Update, "source cache removed");
    let lines = lines(&root, &log);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("DEBUG scan"));
}

#[test]
fn a_line_marked_once_is_written_once_per_session() {
    let (root, log) = logger(LogFilter::default());
    for preset in ["Default", "Default", "Work"] {
        log.event(Level::Warn, Area::Presets, "preset icon unknown")
            .field("preset", preset)
            .once();
    }
    log.event(Level::Warn, Area::Presets, "preset icon unknown")
        .field("preset", "Default");
    assert_eq!(lines(&root, &log).len(), 3);
}

#[test]
fn a_quit_is_not_a_failure() {
    let (root, log) = logger(LogFilter::parse("debug"));
    let quit = crate::shutdown::Shutdown::error();
    let failed = AppError::storage();
    for error in [&quit, &failed] {
        log.failure(
            Level::Warn,
            Area::Update,
            ("check failed", "check stopped at quit"),
            error,
        );
    }
    let lines = lines(&root, &log);
    assert!(
        lines[0].contains("DEBUG update       check stopped at quit"),
        "{}",
        lines[0]
    );
    assert!(!lines[0].contains("diag="));
    assert!(lines[1].contains(&format!(
        "WARN  update       check failed diag={}",
        failed.diagnostic_id
    )));
}

#[test]
fn credentials_and_home_never_reach_fields() {
    let (root, log) = logger(LogFilter::default());
    log.event(Level::Warn, Area::Update, "source check failed")
        .field(
            "url",
            "https://user:secret@example.test/a?token=hidden&x=ok",
        )
        .field(
            "cause",
            "/private/home/a Bearer abc123\nAuthorization: Basic secret",
        );
    let lines = lines(&root, &log);
    let line = &lines[0];
    for secret in [
        "/private/home",
        "user:secret",
        "hidden",
        "abc123",
        "Basic secret",
    ] {
        assert!(!line.contains(secret), "{line}");
    }
    assert!(
        line.contains(r#"url="https://[redacted]@example.test/a?token=[redacted]&x=ok""#),
        "{line}"
    );
    assert!(
        line.contains(r#"cause="~/a Bearer [redacted]\nAuthorization: [redacted]""#),
        "{line}"
    );
    assert!(Regex::new(LINE).unwrap().is_match(line), "{line}");
}

#[test]
fn a_full_queue_sheds_debug_lines_and_counts_them() {
    let (root, log) = logger(LogFilter::parse("scan=debug"));
    for index in 0..3000 {
        if index % 100 == 0 {
            log.event(Level::Info, Area::Scan, "finished")
                .field("count", index);
        }
        log.event(Level::Debug, Area::Scan, "location absent")
            .field("count", index);
    }
    let lines = lines(&root, &log);
    let count = |message: &str| lines.iter().filter(|line| line.contains(message)).count();
    assert_eq!(count(" finished "), 30, "no INFO line is lost");
    let dropped: usize = lines
        .iter()
        .filter(|line| line.contains("WARN  session      debug lines dropped count="))
        .map(|line| line.rsplit('=').next().unwrap().parse::<usize>().unwrap())
        .sum();
    assert_eq!(count(" location absent ") + dropped, 3000);
}

#[test]
fn durations_read_at_a_glance() {
    let took = |duration| Took(duration).to_string();
    assert_eq!(took(Duration::from_millis(840)), "840ms");
    assert_eq!(took(Duration::from_millis(2_940)), "2.9s");
    assert_eq!(took(Duration::from_secs(192)), "3m12s");
    assert_eq!(took(Duration::from_secs(13_020)), "3h37m");
}
