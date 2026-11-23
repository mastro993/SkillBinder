//! A command for dispatch tests: one that answers questions from a queue, and a way to run a
//! future that never waits.

use super::Peer;
use skillbinder_proto::command_line::{Question, Reply};
use std::future::Future;
use std::task::{Context, Poll, Waker};

/// Runs `future`, which never waits here: its answers are queued before it asks.
pub(super) fn now<T>(future: impl Future<Output = T>) -> T {
    let mut future = std::pin::pin!(future);
    match future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("the future waits"),
    }
}

/// A command that answers `answers` in turn, and the replies it was sent.
pub(super) fn command(answers: &[bool]) -> (Peer, async_channel::Receiver<Reply>) {
    let (replies, replied) = async_channel::unbounded();
    let (answer, received) = async_channel::unbounded();
    for yes in answers {
        let _ = answer.try_send(*yes);
    }
    // A command that answered everything it will, then went.
    drop(answer);
    (
        Peer {
            replies,
            answers: received,
        },
        replied,
    )
}

/// The questions the command was asked.
pub(super) fn asked(replied: &async_channel::Receiver<Reply>) -> Vec<String> {
    std::iter::from_fn(|| replied.try_recv().ok())
        .filter_map(|reply| match reply {
            Reply::Ask(Question { prompt }) => Some(prompt),
            _ => None,
        })
        .collect()
}
