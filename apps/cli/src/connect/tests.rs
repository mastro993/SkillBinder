use super::{Conversation, exchange, receive, send_line};
use interprocess::local_socket::prelude::*;
use interprocess::local_socket::{GenericFilePath, ListenerOptions};
use skillbinder_proto::command_line::{
    Answer, Change, Changed, CommandError, Consent, Endpoint, ErrorCode, Failure, Greeting, Hello,
    Output, PROTOCOL, Progress, Question, Reply, Request,
};
use std::io::BufReader;

/// What the test app does after the greeting: send a reply, or read the command's answer.
enum Step {
    Send(Box<Reply>),
    Read,
}

/// What the test app read.
struct Heard {
    hello: Hello,
    request: Option<Request>,
    answers: Vec<bool>,
}

/// A test app that answers one connection with `greeting`, then, when it welcomes, reads the
/// request and plays `steps`.
fn serve_once(
    endpoint: &Endpoint,
    greeting: Greeting,
    steps: Vec<Step>,
) -> std::io::Result<std::thread::JoinHandle<std::io::Result<Heard>>> {
    let Endpoint::Socket(path) = endpoint else {
        unreachable!("Unix gets a socket");
    };
    let listener = ListenerOptions::new()
        .name(path.as_path().to_fs_name::<GenericFilePath>()?)
        .create_sync()?;
    Ok(std::thread::spawn(move || {
        let stream = listener.accept()?;
        let mut reader = BufReader::new(&stream);
        let hello = receive(&mut reader)?;
        let welcome = matches!(greeting, Greeting::Welcome { .. });
        send_line(&stream, &greeting)?;
        let mut heard = Heard {
            hello,
            request: None,
            answers: Vec::new(),
        };
        if welcome {
            heard.request = Some(receive(&mut reader)?);
        }
        for step in steps {
            match step {
                Step::Send(reply) => send_line(&stream, &*reply)?,
                Step::Read => heard.answers.push(receive::<Answer>(&mut reader)?.yes),
            }
        }
        Ok(heard)
    }))
}

/// A user who answers `answer` to every question.
#[derive(Default)]
struct User {
    answer: bool,
    sent: usize,
    progress: Vec<(usize, usize)>,
    asked: Vec<String>,
}

impl Conversation for User {
    fn sent(&mut self, _: &Request) {
        self.sent += 1;
    }

    fn progress(&mut self, progress: &Progress) {
        self.progress.push((progress.done, progress.total));
    }

    fn ask(&mut self, question: &Question) -> bool {
        self.asked.push(question.prompt.clone());
        self.answer
    }
}

fn joined(app: std::thread::JoinHandle<std::io::Result<Heard>>) -> anyhow::Result<Heard> {
    Ok(app
        .join()
        .map_err(|_| anyhow::anyhow!("the test app panicked"))??)
}

#[cfg(unix)]
#[test]
fn another_protocol_exits_5_naming_repair() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let endpoint = Endpoint::of(root.path(), root.path());
    let mismatch = Greeting::Mismatch {
        app_version: "9.9.9".into(),
    };
    let app = serve_once(&endpoint, mismatch, Vec::new())?;
    let Ok(Err(Failure { error, .. })) =
        exchange(&endpoint, &Request::Status, &mut User::default())
    else {
        panic!("a mismatched app refuses the request");
    };
    assert_eq!(joined(app)?.hello.protocol, PROTOCOL);
    assert_eq!(
        (error.code, error.code.exit_code()),
        (ErrorCode::Unavailable, 5)
    );
    assert_eq!(
        error.message,
        "This `skillbinder` doesn't match SkillBinder 9.9.9."
    );
    assert_eq!(
        error.hint.as_deref(),
        Some("Repair it in Settings › General.")
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_switch_shows_progress_asks_and_exits_3_with_what_landed() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let endpoint = Endpoint::of(root.path(), root.path());
    let changed = Changed {
        message: "Work is active: 1 added, 0 removed.".into(),
        ..Changed::default()
    };
    let steps = vec![
        Step::Send(Box::new(Reply::Progress(Progress {
            activity: "Switching to Work".into(),
            done: 1,
            total: 2,
        }))),
        Step::Send(Box::new(Reply::Ask(Question {
            prompt: "Replace your copy of pdf in ~/.claude/skills?".into(),
        }))),
        Step::Read,
        Step::Send(Box::new(Reply::Outcome(Err(Failure {
            error: CommandError::new(ErrorCode::NeedsAnswer, "1 copy needs your answer"),
            result: Some(Box::new(Output::Changed(changed))),
        })))),
    ];
    let welcome = Greeting::Welcome {
        version: "0.1.0".into(),
    };
    let app = serve_once(&endpoint, welcome, steps)?;
    let request = Request::Change {
        change: Change::ActivatePreset {
            preset: "Work".into(),
        },
        consent: Consent::default(),
    };
    let mut user = User::default();
    let Ok(Err(Failure { error, result })) = exchange(&endpoint, &request, &mut user) else {
        panic!("the copy waits");
    };
    let heard = joined(app)?;
    assert_eq!(heard.request, Some(request));
    assert_eq!(heard.answers, [false], "n keeps the copy waiting");
    assert_eq!((user.sent, user.progress), (1, vec![(1, 2)]));
    assert_eq!(
        user.asked,
        ["Replace your copy of pdf in ~/.claude/skills?"]
    );
    assert_eq!(error.code.exit_code(), 3);
    assert!(
        matches!(result.as_deref(), Some(Output::Changed(_))),
        "what landed comes back"
    );
    Ok(())
}

#[test]
fn no_app_is_not_running() -> anyhow::Result<()> {
    let root = tempfile::tempdir()?;
    let endpoint = Endpoint::of(root.path(), root.path());
    assert!(exchange(&endpoint, &Request::Status, &mut User::default()).is_err());
    Ok(())
}
