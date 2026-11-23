//! The terminal while a request runs: its progress on one stderr line, its questions asked on
//! stderr and answered on stdin, and Ctrl-C, which stops the command waiting but leaves the change
//! running in the app (spec rule 3).

use crate::connect::Conversation;
use skillbinder_proto::command_line::{Progress, Question, Request};
use std::io::{BufRead as _, Write as _};

pub(crate) struct Console {
    /// Progress shows only in a terminal.
    terminal: bool,
    /// A progress line is open.
    progress: bool,
}

impl Console {
    pub(crate) fn new(terminal: bool) -> Self {
        Self {
            terminal,
            progress: false,
        }
    }

    /// Ends an open progress line, so what follows starts on its own.
    pub(crate) fn end_progress(&mut self) {
        if std::mem::take(&mut self.progress) {
            eprintln!();
        }
    }
}

impl Conversation for Console {
    fn sent(&mut self, request: &Request) {
        if matches!(request, Request::Change { .. }) {
            // Only the waiting stops; the app finishes the change as if its window had closed.
            let _ = ctrlc::set_handler(|| {
                eprintln!("\nStill running in SkillBinder.");
                std::process::exit(130);
            });
        }
    }

    fn progress(&mut self, progress: &Progress) {
        if self.terminal {
            let Progress {
                activity,
                done,
                total,
            } = progress;
            eprint!("\r{activity}… {done}/{total}");
            self.progress = true;
        }
    }

    fn ask(&mut self, question: &Question) -> bool {
        self.end_progress();
        eprint!("{} [y/N] ", question.prompt);
        let _ = std::io::stderr().flush();
        let mut answer = String::new();
        if std::io::stdin().lock().read_line(&mut answer).is_err() {
            return false;
        }
        is_yes(&answer)
    }
}

/// `y` or `yes`, in any case; anything else, or nothing, is no.
fn is_yes(answer: &str) -> bool {
    matches!(answer.trim().to_lowercase().as_str(), "y" | "yes")
}

#[cfg(test)]
mod tests {
    use super::is_yes;

    #[test]
    fn only_y_or_yes_answers_yes() {
        for yes in ["y", "Y\n", " yes ", "YES"] {
            assert!(is_yes(yes), "{yes:?}");
        }
        for no in ["", "\n", "n", "no", "yep", "sure"] {
            assert!(!is_yes(no), "{no:?}");
        }
    }
}
