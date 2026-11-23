//! A tool call sends the command's request to the app, as the command does, on a blocking thread:
//! progress goes to the client as notifications, and a cancelled call stops waiting while the app
//! finishes (spec rule 6). Questions go to the user through the client (rule 3): during the call
//! before protocol 2026-07-28, and from 2026-07-28 as an input-required result the client answers
//! by calling again. A client that can't ask gets `needs_answer`, for the agent to ask.

mod asking;

use super::Server;
use super::inputs::{Agreed, ResponseFormat};
use super::results;
use super::session::App;
use crate::connect::{self, Conversation};
use asking::{Asked, Asking, input_required, said_yes, yes};
use rmcp::handler::server::common::FromContextPart;
use rmcp::handler::server::tool::{InputResponses, RequestState, ToolCallContext};
use rmcp::model::{
    self, CallToolResponse, CallToolResult, ContentBlock, ProgressNotificationParam,
};
use rmcp::{ErrorData, RoleServer, service::RequestContext};
use skillbinder_proto::command_line::{
    Change, Changed, CommandError, Consent, ErrorCode, Failure, Output, Permission, Progress,
    Question, Request,
};
use std::sync::Arc;

/// What a tool call brings beside its arguments.
pub(crate) struct Turn {
    context: RequestContext<RoleServer>,
    /// What the last round asked, when the client calls again with the user's answers.
    state: Option<String>,
    answers: Option<model::InputResponses>,
}

impl<S> FromContextPart<ToolCallContext<'_, S>> for Turn {
    fn from_context_part(context: &mut ToolCallContext<'_, S>) -> Result<Self, ErrorData> {
        let RequestState(state) = RequestState::from_context_part(context)?;
        let InputResponses(answers) = InputResponses::from_context_part(context)?;
        let context = RequestContext::from_context_part(context)?;
        Ok(Self {
            context,
            state,
            answers,
        })
    }
}

/// The app's answer, unless the call was cancelled first.
type Sent = Option<Result<Output, Failure>>;

impl Server {
    /// Answers a reading request.
    pub(super) async fn read(
        &self,
        request: Request,
        format: ResponseFormat,
        turn: Turn,
    ) -> Result<CallToolResponse, ErrorData> {
        let sent = self.send(request, &turn.context, Asking::Never, None).await;
        Ok(match sent {
            Some(outcome) => results::result(outcome, format).into(),
            None => stopped(),
        })
    }

    /// Runs `change`, with what the user agreed to.
    pub(super) async fn change(
        &self,
        change: Result<Change, CommandError>,
        agreed: Agreed,
        turn: Turn,
    ) -> Result<CallToolResponse, ErrorData> {
        let change = match change {
            Ok(change) => change,
            Err(error) => return Ok(results::error(&error.into()).into()),
        };
        let asking = Asking::of(&turn.context);
        let permission = |agreed: bool| match (agreed, asking) {
            (true, _) => Permission::Yes,
            (false, Asking::During) => Permission::Ask,
            (false, Asking::Retry | Asking::Never) => Permission::No,
        };
        let consent = Consent {
            replace: permission(agreed.replace),
            confirm: permission(agreed.confirm),
        };
        let (sent, asked) = match turn.state.as_deref().map(Asked::read) {
            None => {
                let request = Request::Change { change, consent };
                (self.send(request, &turn.context, asking, None).await, None)
            }
            Some(Ok(asked)) => {
                let sent = self.answer(change, consent, &asked, &turn).await;
                (sent, Some(asked))
            }
            Some(Err(error)) => return Err(error),
        };
        let Some(outcome) = sent else {
            return Ok(stopped());
        };
        let asks = |failure: &Failure| match asked {
            None => true,
            // A confirmed change may still leave copies to ask about.
            Some(Asked::Confirm { .. }) => waiting(failure) > 0,
            // The user answered for these copies; those they said no to wait.
            Some(Asked::Copies { .. }) => false,
        };
        let outcome = match (outcome, asking) {
            (Err(failure), Asking::Retry)
                if failure.error.code == ErrorCode::NeedsAnswer && asks(&failure) =>
            {
                return Ok(input_required(failure).into());
            }
            (outcome, asking) => outcome.map_err(|failure| needs_answer(failure, asking)),
        };
        Ok(results::result(outcome, ResponseFormat::Concise).into())
    }

    /// Runs `changes` in turn, as answering several copies runs one command each; one that fails
    /// stops the rest, beside what the earlier ones did.
    pub(super) async fn changes(
        &self,
        changes: Result<Vec<Change>, CommandError>,
        turn: Turn,
    ) -> Result<CallToolResponse, ErrorData> {
        let changes = match changes {
            Ok(changes) => changes,
            Err(error) => return Ok(results::error(&error.into()).into()),
        };
        let mut done = Vec::new();
        for change in changes {
            let request = Request::Change {
                change,
                consent: Consent::default(),
            };
            let Some(outcome) = self.send(request, &turn.context, Asking::Never, None).await else {
                return Ok(stopped());
            };
            match outcome {
                Ok(Output::Changed(changed)) => done.push(changed.message),
                Ok(_) => {}
                Err(mut failure) => {
                    if failure.result.is_none() && !done.is_empty() {
                        let changed = Changed::said(done.join(" "));
                        failure.result = Some(Box::new(Output::Changed(changed)));
                    }
                    return Ok(results::error(&failure).into());
                }
            }
        }
        let changed = Output::Changed(Changed::said(done.join(" ")));
        Ok(results::result(Ok(changed), ResponseFormat::Concise).into())
    }

    /// Settles what the last round asked with the user's answers: replaces the copies they said
    /// yes to, or runs the change they confirmed, unless its question has changed since.
    async fn answer(&self, change: Change, consent: Consent, asked: &Asked, turn: &Turn) -> Sent {
        match asked {
            Asked::Copies { copies, changed } => {
                let mut changed = changed.clone();
                let mut stopped = None;
                for (index, copy) in copies.iter().enumerate() {
                    if stopped.is_none() && said_yes(turn.answers.as_ref(), index) {
                        let change = Change::ReplaceCopy {
                            skill: copy.skill.clone(),
                            directory: Some(copy.directory.clone()),
                        };
                        let request = Request::Change {
                            change,
                            consent: Consent::default(),
                        };
                        match self
                            .send(request, &turn.context, Asking::Never, None)
                            .await?
                        {
                            Ok(_) => continue,
                            Err(failure) => stopped = Some(failure.error),
                        }
                    }
                    changed.waiting.push(copy.clone());
                }
                let waiting = !changed.waiting.is_empty();
                let output = Output::Changed(changed);
                // A copy left waiting gets its words from `needs_answer`, as any does.
                let error = match stopped {
                    Some(error) => error,
                    None if waiting => CommandError::new(ErrorCode::NeedsAnswer, ""),
                    None => return Some(Ok(output)),
                };
                Some(Err(Failure {
                    error,
                    result: Some(Box::new(output)),
                }))
            }
            Asked::Confirm { .. } if !said_yes(turn.answers.as_ref(), 0) => Some(Err(
                CommandError::new(ErrorCode::NeedsAnswer, "Nothing changed.").into(),
            )),
            Asked::Confirm { prompt } => {
                let consent = Consent {
                    confirm: Permission::Ask,
                    ..consent
                };
                let request = Request::Change { change, consent };
                let answered = Some(prompt.clone());
                let sent = self
                    .send(request, &turn.context, Asking::Never, answered)
                    .await?;
                // The app asked something else, so the user's yes was to a library since changed.
                Some(sent.map_err(|failure| {
                    match failure {
                        Failure {
                            error:
                                CommandError {
                                    code: ErrorCode::NeedsAnswer,
                                    ..
                                },
                            result: None,
                        } => CommandError::new(
                            ErrorCode::Conflict,
                            "The library changed after the user answered, so nothing changed.",
                        )
                        .hint("Call again to ask them afresh.")
                        .into(),
                        failure => failure,
                    }
                }))
            }
        }
    }

    /// Sends `request` to the app, starting it when none answers, and waits for its outcome.
    /// `answered` is the question the user already said yes to.
    async fn send(
        &self,
        request: Request,
        context: &RequestContext<RoleServer>,
        asking: Asking,
        answered: Option<String>,
    ) -> Sent {
        let mut talk = Talk {
            runtime: tokio::runtime::Handle::current(),
            context: context.clone(),
            elicits: asking == Asking::During,
            answered,
            app: self.app.clone(),
        };
        let app = self.app.clone();
        let sent = tokio::task::spawn_blocking(move || {
            connect::send(app.data(), &request, app.isolated(), &mut talk)
        });
        tokio::select! {
            joined = sent => Some(joined.unwrap_or_else(|error| {
                Err(connect::unavailable(format!("The request stopped: {error}")).into())
            })),
            () = context.ct.cancelled() => None,
        }
    }
}

/// What a cancelled call returns, which the client never reads.
fn stopped() -> CallToolResponse {
    let text = "Stopped waiting. SkillBinder finishes the change.";
    CallToolResult::error(vec![ContentBlock::text(text)]).into()
}

/// The command's `needs_answer` in words for the agent: copies a change left waiting are
/// answered with resolve_copies, since calling the change again finds it done, and a deletion
/// the client couldn't ask about is called again with `confirm` once the user agreed.
fn needs_answer(failure: Failure, asking: Asking) -> Failure {
    if failure.error.code != ErrorCode::NeedsAnswer {
        return failure;
    }
    let error = match waiting(&failure) {
        0 if asking == Asking::Never => {
            CommandError::new(ErrorCode::NeedsAnswer, failure.error.message)
                .hint("Ask the user, then call again with confirm: true.")
        }
        0 => failure.error,
        1 => CommandError::new(ErrorCode::NeedsAnswer, "1 copy needs the user's answer.").hint(
            "Ask the user, then call resolve_copies with action replace, or ask them to review \
             it in SkillBinder.",
        ),
        count => CommandError::new(
            ErrorCode::NeedsAnswer,
            format!("{count} copies need the user's answer."),
        )
        .hint(
            "Ask the user, then call resolve_copies with action replace for each, or ask them \
             to review them in SkillBinder.",
        ),
    };
    Failure { error, ..failure }
}

/// How many copies `failure` left waiting.
fn waiting(failure: &Failure) -> usize {
    match failure.result.as_deref() {
        Some(Output::Changed(changed)) => changed.waiting.len(),
        _ => 0,
    }
}

/// The client's side of one request, on its blocking thread.
struct Talk {
    runtime: tokio::runtime::Handle,
    context: RequestContext<RoleServer>,
    /// The app's questions go to the user during the call.
    elicits: bool,
    /// The question the user already said yes to, on a retry; any other is a change made since.
    answered: Option<String>,
    app: Arc<App>,
}

impl Conversation for Talk {
    /// Holds the app before the request goes, so its connection can't close, and an app with
    /// no icon quit, before the app counts the hold.
    fn welcomed(&mut self) {
        self.app.hold();
    }

    fn sent(&mut self, _: &Request) {}

    fn progress(&mut self, progress: &Progress) {
        let Some(token) = self.context.meta.get_progress_token() else {
            return;
        };
        let count = |count: usize| f64::from(u32::try_from(count).unwrap_or(u32::MAX));
        let mut param = ProgressNotificationParam::new(token, count(progress.done));
        param.total = Some(count(progress.total));
        param.message = Some(format!("{}…", progress.activity));
        let _ = self
            .runtime
            .block_on(self.context.peer.notify_progress(param));
    }

    fn ask(&mut self, question: &Question) -> bool {
        if let Some(answered) = &self.answered {
            return *answered == question.prompt;
        }
        if !self.elicits {
            return false;
        }
        let asked = self
            .context
            .peer
            .create_elicitation(asking::form(&question.prompt));
        let cancelled = self.context.ct.cancelled();
        self.runtime.block_on(async {
            tokio::select! {
                answer = asked => answer.is_ok_and(|answer| yes(&answer)),
                () = cancelled => false,
            }
        })
    }
}
