//! Settings › Library & sync: the library folder with Reveal, whether SkillBinder can write it,
//! and the Git row (onboarding S3) and the remote, both facts.

use super::super::NativeView;
use super::super::controls::agent_detail;
use super::super::git_setup::GitButton;
use super::catalog::{Group, SettingRow};
use gpui_kit::{AnyElement, Context};
use skillbinder_design::Button;
use std::path::PathBuf;

/// The Git row's line while Git is found.
const GIT_FOUND: &str = "Syncs your library and updates skills from their sources.";

impl NativeView {
    pub(super) fn library_groups(&self, cx: &mut Context<Self>) -> Vec<Group> {
        let bootstrap = &self.snapshot.bootstrap;
        let path = bootstrap.library_path.clone();
        let folder = SettingRow::new("settings-library-folder", "Library folder")
            .subtitle(
                path.clone(),
                agent_detail("settings-library-path".into(), path, true, cx),
            )
            .keywords(&["path", "location", "managed library"])
            .control(
                Button::new("settings-reveal-library")
                    .small()
                    .label("Reveal")
                    .on_click(cx.listener(|view, _, _, cx| {
                        cx.reveal_path(&PathBuf::from(&view.snapshot.bootstrap.library_path));
                    })),
            );
        let writable = if bootstrap.storage_writable {
            (
                "Writable",
                "SkillBinder can save changes to the library folder.",
            )
        } else {
            (
                "Read-only",
                "SkillBinder can't save changes to the library folder.",
            )
        };
        let storage = SettingRow::new("settings-storage", "Storage")
            .detail(writable.1)
            .keywords(&["writable", "permissions", "disk"])
            .fact("settings-storage-value", writable.0);
        let remote = self
            .snapshot
            .sync
            .remote
            .as_ref()
            .map_or("Local only", |remote| remote.url.as_str());
        let remote = SettingRow::new("settings-remote", "Remote")
            .detail("Set it up in Sync.")
            .keywords(&["repository", "github", "url", "push", "pull"])
            .fact("settings-remote-value", remote.to_owned());
        vec![
            Group::titled("Library", vec![folder, storage]),
            Group::titled("Sync", vec![self.git_setting(cx), remote]),
        ]
    }

    /// Git's version, or "Not installed" with the step's primary action and its status line,
    /// and the package manager's command on Linux under them (S3).
    fn git_setting(&self, cx: &mut Context<Self>) -> SettingRow {
        let row = SettingRow::new("settings-git-row", "Git").keywords(&["install", "version"]);
        let facts = self.git_facts();
        if let Some(version) = facts.found {
            return row
                .detail(GIT_FOUND)
                .fact("settings-git", version.to_owned());
        }
        let actions = facts.actions();
        let primary = if actions.install {
            Some(GitButton::Install)
        } else {
            actions.download_primary.then_some(GitButton::Download)
        };
        let value = facts.outdated.unwrap_or("Not installed");
        let under: Vec<AnyElement> = self
            .git_status_line("settings-git-line")
            .into_iter()
            .chain(
                actions
                    .command
                    .map(|command| self.git_command_box(command, cx)),
            )
            .collect();
        let row = row
            .detail(format!("{value}. Sync and skill updates need Git."))
            .under(under);
        match primary {
            Some(button) => row.control(self.git_button(button, true, false, cx)),
            None => row,
        }
    }
}
