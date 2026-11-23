//! How a result prints (spec rule 9): text on stdout and errors on stderr, or with `--json`
//! exactly one document on stdout, `{"ok": true, "result": …}` or `{"ok": false, "error": …}`.

use serde_json::json;
use skillbinder_proto::SyncState;
use skillbinder_proto::command_line::{CommandError, Output, StatusOutput};
use std::process::ExitCode;

/// Prints `outcome` and returns its exit code (spec rule 10).
pub(crate) fn outcome(outcome: Result<Output, CommandError>, json: bool) -> ExitCode {
    let output = match outcome {
        Ok(output) => output,
        Err(error) => return failure(&error, json),
    };
    if json {
        let result = match &output {
            Output::Status(status) => json!(status),
        };
        println!("{}", json!({ "ok": true, "result": result }));
    } else {
        match &output {
            Output::Status(status) => print!("{}", status_text(status)),
        }
    }
    ExitCode::SUCCESS
}

/// Prints `error` and returns its exit code.
pub(crate) fn failure(error: &CommandError, json: bool) -> ExitCode {
    if json {
        println!("{}", json!({ "ok": false, "error": error }));
    } else {
        eprintln!("{}", error.message);
        if let Some(hint) = &error.hint {
            eprintln!("{hint}");
        }
    }
    ExitCode::from(error.code.exit_code())
}

fn status_text(status: &StatusOutput) -> String {
    let StatusOutput {
        version,
        onboarding_finished,
        active_preset,
        skills,
        active,
        updates,
        attention,
        sync,
        running,
    } = status;
    let mut text = format!("SkillBinder {version}\n");
    if !onboarding_finished {
        text.push_str("Setup isn't finished. Open SkillBinder to finish setting up.\n");
        return text;
    }
    if let Some(preset) = active_preset {
        text.push_str(&format!("Active preset: {preset}\n"));
    }
    let skills = match skills {
        1 => "1 skill".to_owned(),
        count => format!("{count} skills"),
    };
    text.push_str(&format!(
        "Library: {skills}, {active} active, {updates} with updates\n"
    ));
    if *attention > 0 {
        text.push_str(&format!("Copies needing your answer: {attention}\n"));
    }
    let sync = match sync {
        SyncState::NotConfigured => "not set up",
        SyncState::Synced => "up to date",
        SyncState::NeedsPull => "needs pull",
        SyncState::NeedsPush => "needs push",
        SyncState::NeedsSync => "needs sync",
    };
    text.push_str(&format!("Sync: {sync}\n"));
    if let Some(running) = running {
        text.push_str(&format!("Running: {running}…\n"));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::status_text;
    use skillbinder_proto::SyncState;
    use skillbinder_proto::command_line::StatusOutput;

    fn status() -> StatusOutput {
        StatusOutput {
            version: "0.1.0".into(),
            onboarding_finished: true,
            active_preset: Some("Work".into()),
            skills: 42,
            active: 12,
            updates: 3,
            attention: 1,
            sync: SyncState::NeedsPush,
            running: Some("Switching to Work".into()),
        }
    }

    #[test]
    fn status_prints_one_fact_per_line() {
        assert_eq!(
            status_text(&status()),
            "SkillBinder 0.1.0\nActive preset: Work\nLibrary: 42 skills, 12 active, 3 with updates\nCopies needing your answer: 1\nSync: needs push\nRunning: Switching to Work…\n"
        );
    }

    #[test]
    fn before_setup_status_says_to_finish_it() {
        let status = StatusOutput {
            onboarding_finished: false,
            ..status()
        };
        assert_eq!(
            status_text(&status),
            "SkillBinder 0.1.0\nSetup isn't finished. Open SkillBinder to finish setting up.\n"
        );
    }
}
