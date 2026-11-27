//! What a tool returns (spec rules 4 and 5): structured content made of the command's own rows,
//! so `--json` and MCP share one schema, beside the command's text for clients that read only
//! text; and a failure as a tool error carrying the command's `CommandError`, with what landed.

use super::inputs::ResponseFormat;
use crate::render::{self, text};
use rmcp::model::{CallToolResult, ContentBlock, JsonObject};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Value, json};
use skillbinder_proto::command_line::{
    AgentRow, Changed, CopyRow, Failure, FolderRow, FoundOutput, FoundRow, HistoryOutput, Output,
    PresetRow, RepositoryRow, ReviewsOutput, SearchOutput, SearchRow, SkillRow, SkillsShAnswer,
    SyncStatusOutput, Unchecked, UpdatesOutput, WorkspaceRow,
};
use std::sync::Arc;

/// The most entries a list returns.
const LIMIT: usize = 200;

/// The fields only `detailed` returns. A schema leaves them optional.
const DETAILS: [&str; 3] = ["description", "source", "recorded"];

/// list_skills' result.
#[derive(Serialize, JsonSchema)]
pub(crate) struct Skills {
    /// The skills, at most 200.
    skills: Vec<SkillRow>,
    /// How many more there are than listed.
    more: usize,
}

/// list_places' result: the one kind it was asked for.
#[derive(Default, Serialize, JsonSchema)]
pub(crate) struct Places {
    /// The presets, at most 200.
    #[serde(skip_serializing_if = "Option::is_none")]
    presets: Option<Vec<PresetRow>>,
    /// The library folders, at most 200.
    #[serde(skip_serializing_if = "Option::is_none")]
    folders: Option<Vec<FolderRow>>,
    /// The copies waiting for the user's answer, at most 200.
    #[serde(skip_serializing_if = "Option::is_none")]
    copies: Option<Vec<CopyRow>>,
    /// The workspaces, at most 200.
    #[serde(skip_serializing_if = "Option::is_none")]
    workspaces: Option<Vec<WorkspaceRow>>,
    /// The agents found on this computer, at most 200.
    #[serde(skip_serializing_if = "Option::is_none")]
    agents: Option<Vec<AgentRow>>,
    /// The user's repositories, at most 200.
    #[serde(skip_serializing_if = "Option::is_none")]
    repositories: Option<Vec<RepositoryRow>>,
    /// How many more there are than listed.
    more: usize,
}

/// find_skills' result: what the scan found, or what the search found.
#[derive(Default, Serialize, JsonSchema)]
pub(crate) struct Finds {
    /// The found skills not in library and not ignored, at most 200, for where found.
    #[serde(skip_serializing_if = "Option::is_none")]
    found: Option<Vec<FoundRow>>,
    /// Every slug ignored on this device, sorted, for where found; ignore_skills with stop
    /// takes them.
    #[serde(skip_serializing_if = "Option::is_none")]
    ignored: Option<Vec<String>>,
    /// The search's results, at most 200, for where explore.
    #[serde(skip_serializing_if = "Option::is_none")]
    results: Option<Vec<SearchRow>>,
    /// How skills.sh answered the search.
    #[serde(skip_serializing_if = "Option::is_none")]
    skills_sh: Option<SkillsShAnswer>,
    /// How many more there are than listed.
    more: usize,
}

/// update_skills' check: the command's `updates` beside `more`.
#[derive(Serialize, JsonSchema)]
pub(crate) struct Checked {
    /// The skills whose source has a newer version, at most 200.
    skills: Vec<SkillRow>,
    /// The skills whose source the check couldn't read.
    unchecked: Vec<Unchecked>,
    /// How many more skills have an update than listed.
    more: usize,
}

/// update_skills' result: check's list, diff's reviews, or what an update or reinstall did.
#[derive(JsonSchema)]
#[serde(untagged)]
#[expect(dead_code, reason = "only its schema is used")]
pub(crate) enum Updates {
    Checked(Checked),
    Reviewed(ReviewsOutput),
    Changed(Changed),
}

/// sync_library's result: what waits to sync, a page of History, or what a change did.
#[derive(JsonSchema)]
#[serde(untagged)]
#[expect(dead_code, reason = "only its schema is used")]
pub(crate) enum Synced {
    Status(SyncStatusOutput),
    History(HistoryOutput),
    Changed(Changed),
}

/// The output schema of a tool returning `T`, with the detail fields optional and without
/// schemars' integer formats such as `uint`, which JSON Schema doesn't define and clients warn
/// about.
pub(crate) fn schema<T: JsonSchema + 'static>() -> Arc<JsonObject> {
    let mut schema = Value::Object(
        rmcp::handler::server::tool::schema_for_output::<T>()
            .as_ref()
            .clone(),
    );
    tidy(&mut schema);
    match schema {
        // A result is always an object, which some clients require the schema to say.
        Value::Object(mut object) => {
            object
                .entry("type")
                .or_insert_with(|| Value::from("object"));
            Arc::new(object)
        }
        _ => unreachable!("a schema stays an object"),
    }
}

fn tidy(schema: &mut Value) {
    match schema {
        Value::Object(object) => {
            if let Some(Value::Array(required)) = object.get_mut("required") {
                required.retain(|field| !DETAILS.iter().any(|detail| field == detail));
            }
            let numeric = |format: &str| {
                format.starts_with("int") || format.starts_with("uint") || format == "double"
            };
            if object
                .get("format")
                .and_then(Value::as_str)
                .is_some_and(numeric)
            {
                object.remove("format");
            }
            object.values_mut().for_each(tidy);
        }
        Value::Array(items) => items.iter_mut().for_each(tidy),
        _ => {}
    }
}

/// The tool result for `outcome`.
pub(crate) fn result(outcome: Result<Output, Failure>, format: ResponseFormat) -> CallToolResult {
    match outcome {
        Ok(output) => success(output, format),
        Err(failure) => error(&failure),
    }
}

fn success(output: Output, format: ResponseFormat) -> CallToolResult {
    let (output, more) = capped(output);
    let mut text = text::output(&output);
    if more > 0 {
        text.push_str(&format!("…and {more} more.\n"));
    }
    let mut value = structured(output, more);
    if format == ResponseFormat::Concise {
        concise(&mut value);
    }
    let mut result = CallToolResult::structured(value);
    result.content = vec![ContentBlock::text(text)];
    result
}

/// A failure as the command's `--json` document has it: `error`, and `result` when part of the
/// change landed, such as copies left `waiting`.
pub(crate) fn error(failure: &Failure) -> CallToolResult {
    let Failure { error, result } = failure;
    let mut value = json!({ "error": error });
    if let Some(output) = result {
        value["result"] = render::result(output);
    }
    let mut result = CallToolResult::structured_error(value);
    result.content = vec![ContentBlock::text(text::failure(failure))];
    result
}

/// `output` with its list cut to [`LIMIT`], and how many it left out.
fn capped(output: Output) -> (Output, usize) {
    fn cut<T>(mut rows: Vec<T>) -> (Vec<T>, usize) {
        let more = rows.len().saturating_sub(LIMIT);
        rows.truncate(LIMIT);
        (rows, more)
    }
    match output {
        Output::Skills(rows) => {
            let (rows, more) = cut(rows);
            (Output::Skills(rows), more)
        }
        Output::Presets(rows) => {
            let (rows, more) = cut(rows);
            (Output::Presets(rows), more)
        }
        Output::Folders(rows) => {
            let (rows, more) = cut(rows);
            (Output::Folders(rows), more)
        }
        Output::Copies(rows) => {
            let (rows, more) = cut(rows);
            (Output::Copies(rows), more)
        }
        Output::Workspaces(rows) => {
            let (rows, more) = cut(rows);
            (Output::Workspaces(rows), more)
        }
        Output::Agents(rows) => {
            let (rows, more) = cut(rows);
            (Output::Agents(rows), more)
        }
        Output::Repositories(rows) => {
            let (rows, more) = cut(rows);
            (Output::Repositories(rows), more)
        }
        Output::Found(FoundOutput { found, ignored }) => {
            let (found, more) = cut(found);
            (Output::Found(FoundOutput { found, ignored }), more)
        }
        Output::Search(SearchOutput { results, skills_sh }) => {
            let (results, more) = cut(results);
            (Output::Search(SearchOutput { results, skills_sh }), more)
        }
        Output::Updates(UpdatesOutput { skills, unchecked }) => {
            let (skills, more) = cut(skills);
            (Output::Updates(UpdatesOutput { skills, unchecked }), more)
        }
        output => (output, 0),
    }
}

/// The structured content: an object, so a list goes under its name beside `more`.
fn structured(output: Output, more: usize) -> Value {
    let places = Places {
        more,
        ..Places::default()
    };
    match output {
        Output::Skills(skills) => json!(Skills { skills, more }),
        Output::Presets(presets) => json!(Places {
            presets: Some(presets),
            ..places
        }),
        Output::Folders(folders) => json!(Places {
            folders: Some(folders),
            ..places
        }),
        Output::Copies(copies) => json!(Places {
            copies: Some(copies),
            ..places
        }),
        Output::Workspaces(workspaces) => json!(Places {
            workspaces: Some(workspaces),
            ..places
        }),
        Output::Agents(agents) => json!(Places {
            agents: Some(agents),
            ..places
        }),
        Output::Repositories(repositories) => json!(Places {
            repositories: Some(repositories),
            ..places
        }),
        Output::Found(FoundOutput { found, ignored }) => json!(Finds {
            found: Some(found),
            ignored: Some(ignored),
            more,
            ..Finds::default()
        }),
        Output::Search(SearchOutput { results, skills_sh }) => json!(Finds {
            results: Some(results),
            skills_sh: Some(skills_sh),
            more,
            ..Finds::default()
        }),
        Output::Updates(UpdatesOutput { skills, unchecked }) => json!(Checked {
            skills,
            unchecked,
            more,
        }),
        Output::Status(_)
        | Output::Skill(_)
        | Output::SyncStatus(_)
        | Output::History(_)
        | Output::Reviews(_)
        | Output::Changed(_) => render::result(&output),
    }
}

/// Leaves out the detail fields of the result and of the rows it lists.
fn concise(value: &mut Value) {
    let Value::Object(object) = value else {
        return;
    };
    for detail in DETAILS {
        object.remove(detail);
    }
    for rows in object.values_mut() {
        if let Value::Array(rows) = rows {
            for row in rows.iter_mut().filter_map(Value::as_object_mut) {
                for detail in DETAILS {
                    row.remove(detail);
                }
            }
        }
    }
}
