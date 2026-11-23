//! Settings › General › Command line tool (ADR 0024): whether the `skillbinder` command works in a
//! new terminal, and what Install, Repair, and Uninstall can do about it.

use std::path::PathBuf;

/// Where `skillbinder` is, or where Install puts it, and what is there.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandLineStatus {
    /// What is there.
    pub state: CommandLineState,
    /// The command a terminal runs: the link, the copy, or on Windows the program in the folder
    /// Install adds to the user's PATH.
    pub path: PathBuf,
}

/// A row of the spec's Settings table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandLineState {
    /// Nothing is there; Install puts it there.
    NotInstalled,
    /// Install put it there, leading to this app; Uninstall removes it.
    Installed,
    /// The Homebrew cask linked it, so `brew` owns it.
    Homebrew,
    /// A link or copy SkillBinder wrote leads to another SkillBinder, such as one moved; Repair
    /// points it at this one.
    OtherApp,
    /// Linux: installed, in a folder missing from the PATH.
    NotOnPath,
    /// A `skillbinder` SkillBinder didn't write, which it never replaces.
    InTheWay,
    /// This copy of SkillBinder can't install the command.
    CannotInstall(CannotInstallCommand),
}

/// Why this copy can't install the command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CannotInstallCommand {
    /// macOS: the app runs outside `/Applications` and `~/Applications`, so a link would break.
    OutsideApplications,
    /// Windows: the setup program didn't install this copy.
    NotSetUp,
    /// Linux: the app doesn't run as its AppImage, so there is no command to copy.
    NotAppImage,
}
