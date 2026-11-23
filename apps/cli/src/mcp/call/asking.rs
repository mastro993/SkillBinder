//! Putting the app's questions to the user through the client (spec rule 3): each is one yes or
//! no form. A client on protocol 2026-07-28 answers them by calling the tool again, with the
//! state of what was asked echoed back.

use rmcp::model::{
    BooleanSchema, ElicitRequest, ElicitRequestParams, ElicitResult, ElicitationAction,
    ElicitationSchema, InputRequest, InputRequiredResult, InputResponses,
    PrimitiveSchemaDefinition, ProtocolVersion,
};
use rmcp::{ErrorData, RoleServer, service::RequestContext};
use serde::{Deserialize, Serialize};
use skillbinder_proto::command_line::{Changed, CopyRow, CopyStatus, Failure, Output};
use std::collections::BTreeMap;

/// How the client can put a question to the user.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Asking {
    /// During the call, before protocol 2026-07-28.
    During,
    /// In an input-required result, answered by calling again (2026-07-28).
    Retry,
    /// It can't, so the agent asks.
    Never,
}

impl Asking {
    pub(super) fn of(context: &RequestContext<RoleServer>) -> Self {
        let forms = context
            .client_capabilities()
            .and_then(|capabilities| capabilities.elicitation)
            .is_some_and(|elicitation| elicitation.form.is_some() || elicitation.url.is_none());
        let stateless = context
            .protocol_version()
            .is_some_and(|version| version.as_str() >= ProtocolVersion::V_2026_07_28.as_str());
        match (forms, stateless) {
            (false, _) => Self::Never,
            (true, true) => Self::Retry,
            (true, false) => Self::During,
        }
    }
}

/// What a round asked, which the client echoes back with the answers. A client could alter it,
/// but it names only copies already waiting and a question the app asks again, so the user's
/// answers still decide.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "asked", rename_all = "snake_case")]
pub(super) enum Asked {
    /// The copies a change left waiting, one question each, and what the change did.
    Copies {
        copies: Vec<CopyRow>,
        changed: Changed,
    },
    /// The app's confirmation, asked before anything changed.
    Confirm { prompt: String },
}

impl Asked {
    pub(super) fn read(state: &str) -> Result<Self, ErrorData> {
        serde_json::from_str(state).map_err(|_| {
            ErrorData::invalid_params("requestState is not one this server sent.", None)
        })
    }
}

/// The questions behind a `needs_answer` failure: one per copy it left waiting, or the
/// confirmation in its message.
pub(super) fn input_required(failure: Failure) -> InputRequiredResult {
    let Failure { error, result } = failure;
    let (asked, prompts) = match result.map(|result| *result) {
        Some(Output::Changed(mut changed)) if !changed.waiting.is_empty() => {
            let copies = std::mem::take(&mut changed.waiting);
            let prompts = copies.iter().map(copy_prompt).collect();
            (Asked::Copies { copies, changed }, prompts)
        }
        _ => {
            let prompt = error.message;
            let asked = Asked::Confirm {
                prompt: prompt.clone(),
            };
            (asked, vec![prompt])
        }
    };
    let requests = prompts
        .iter()
        .enumerate()
        .map(|(index, prompt)| {
            let request = ElicitRequest::new(form(prompt));
            (key(index), InputRequest::Elicitation(request))
        })
        .collect();
    InputRequiredResult::new(Some(requests), serde_json::to_string(&asked).ok())
}

/// The user said yes to the `index`th question of the last round.
pub(super) fn said_yes(answers: Option<&InputResponses>, index: usize) -> bool {
    answers
        .and_then(|answers| answers.get(&key(index)))
        .and_then(|answer| serde_json::from_value::<ElicitResult>(answer.clone()).ok())
        .is_some_and(|answer| yes(&answer))
}

/// Accepted with yes ticked; declining, cancelling, or leaving it unticked is no.
pub(super) fn yes(answer: &ElicitResult) -> bool {
    answer.action == ElicitationAction::Accept
        && answer
            .content
            .as_ref()
            .and_then(|content| content.get("yes"))
            .and_then(serde_json::Value::as_bool)
            == Some(true)
}

/// A form asking `prompt`, with one yes or no.
pub(super) fn form(prompt: &str) -> ElicitRequestParams {
    let yes = PrimitiveSchemaDefinition::Boolean(BooleanSchema::new().title("Yes"));
    let mut schema = ElicitationSchema::new(BTreeMap::from([("yes".to_owned(), yes)]));
    schema.required = Some(vec!["yes".to_owned()]);
    ElicitRequestParams::FormElicitationParams {
        meta: None,
        message: prompt.to_owned(),
        requested_schema: schema,
    }
}

fn key(index: usize) -> String {
    format!("answer-{index}")
}

/// The app's question for a waiting copy: an edited copy of a skill no longer active there goes,
/// any other is replaced.
fn copy_prompt(copy: &CopyRow) -> String {
    let CopyRow {
        skill, directory, ..
    } = copy;
    match copy.state {
        CopyStatus::EditedInactive => {
            format!("Remove your edited copy of {skill} from {directory}?")
        }
        _ => format!("Replace your copy of {skill} in {directory}?"),
    }
}
