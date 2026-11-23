//! What a tool returns (spec rules 4 and 5): structured content made of the command's own rows,
//! so `--json` and MCP share one schema, beside the command's text for clients that read only
//! text; and a failure as a tool error carrying the command's `CommandError`, with what landed.

use super::inputs::ResponseFormat;
use crate::render::{self, text};
use rmcp::model::{CallToolResult, ContentBlock, JsonObject};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Value, json};
use skillbinder_proto::command_line::{CopyRow, Failure, FolderRow, Output, PresetRow, SkillRow};
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
    /// How many more there are than listed.
    more: usize,
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
        Value::Object(object) => Arc::new(object),
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
        Output::Status(_) | Output::Skill(_) | Output::Changed(_) => render::result(&output),
        // No tool returns these yet; their tools list them under their own names.
        Output::Workspaces(_)
        | Output::Agents(_)
        | Output::Found(_)
        | Output::Search(_)
        | Output::Repositories(_)
        | Output::Updates(_) => render::result(&output),
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
