//! How a result prints (spec rule 9): text on stdout and errors on stderr, or with `--json`
//! exactly one document on stdout, `{"ok": true, "result": …}`, `{"ok": false, "error": …}`, or,
//! when a change partly landed, `{"ok": false, "error": …, "result": …}`.

pub(crate) mod text;

use serde_json::{Value, json};
use skillbinder_proto::command_line::{Failure, Output};
use std::process::ExitCode;

/// Prints `outcome` and returns its exit code (spec rule 10).
pub(crate) fn outcome(outcome: Result<Output, Failure>, json: bool) -> ExitCode {
    let output = match outcome {
        Ok(output) => output,
        Err(failure) => return self::failure(&failure, json),
    };
    if json {
        println!("{}", json!({ "ok": true, "result": result(&output) }));
    } else {
        print!("{}", text::output(&output));
    }
    ExitCode::SUCCESS
}

/// Prints `failure`, with what it changed, and returns its exit code.
pub(crate) fn failure(failure: &Failure, json: bool) -> ExitCode {
    let Failure { error, result } = failure;
    if json {
        let mut document = json!({ "ok": false, "error": error });
        if let Some(output) = result {
            document["result"] = self::result(output);
        }
        println!("{document}");
    } else {
        if let Some(output) = result {
            print!("{}", text::output(output));
        }
        eprint!("{}", text::failure(failure));
    }
    ExitCode::from(error.code.exit_code())
}

/// The document's `result`: the output itself, without the name of its kind.
pub(crate) fn result(output: &Output) -> Value {
    match output {
        Output::Status(status) => json!(status),
        Output::Skills(skills) => json!(skills),
        Output::Skill(skill) => json!(skill),
        Output::Presets(presets) => json!(presets),
        Output::Folders(folders) => json!(folders),
        Output::Copies(copies) => json!(copies),
        Output::Workspaces(workspaces) => json!(workspaces),
        Output::Agents(agents) => json!(agents),
        Output::Found(found) => json!(found),
        Output::Search(search) => json!(search),
        Output::Repositories(rows) => json!(rows),
        Output::Updates(updates) => json!(updates),
        Output::Changed(changed) => json!(changed),
    }
}

#[cfg(test)]
mod tests {
    use super::result;
    use skillbinder_proto::command_line::{Changed, CopyRow, CopyStatus, Output};

    #[test]
    fn a_partial_result_keeps_its_fields_as_the_design_shows() {
        let changed = Changed {
            message: "Work is active: 0 added, 0 removed.".into(),
            activated: Some("Work".into()),
            waiting: vec![CopyRow {
                skill: "pdf".into(),
                directory: "~/.claude/skills".into(),
                state: CopyStatus::Conflict,
                reason: None,
            }],
            ..Changed::default()
        };
        let value = result(&Output::Changed(changed));
        assert_eq!(value["activated"], "Work");
        assert_eq!(value["waiting"][0]["state"], "conflict");
        assert_eq!(value["waiting"][0]["directory"], "~/.claude/skills");
    }
}
