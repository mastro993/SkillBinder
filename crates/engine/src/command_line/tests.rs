use super::copy::Copy;
use super::link::Link;
use super::user_path::{contains, with, without};
use super::{CommandLine, Place};
use crate::logging::Logger;
use skillbinder_proto::{CannotInstallCommand, CommandLineState};
use std::{fs, path::Path, sync::Arc};
use tempfile::TempDir;

fn command_line(root: &Path, place: Place) -> CommandLine {
    let log = Logger::start(root.join("logs"), root).unwrap_or_else(|_| panic!("logs start"));
    CommandLine {
        place: Arc::new(place),
        log,
    }
}

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().unwrap_or(path)).unwrap_or_else(|_| panic!("folder"));
    fs::write(path, contents).unwrap_or_else(|_| panic!("write {}", path.display()));
}

#[cfg(unix)]
mod link {
    use super::*;
    use std::os::unix::fs::symlink;

    /// This app's bundle in a temporary Applications, `/usr/local/bin` and the Homebrew prefixes
    /// inside the same folder.
    fn link(root: &Path) -> Link {
        let command = root.join("Applications/SkillBinder.app/Contents/MacOS/skillbinder-cli");
        write(&command, "this app");
        fs::create_dir_all(root.join("usr/local/bin")).unwrap_or_else(|_| panic!("bin"));
        Link {
            command: Some(command),
            link: root.join("usr/local/bin/skillbinder"),
            prefixes: vec![root.join("opt/homebrew"), root.join("usr/local")],
        }
    }

    fn state(link: &Link) -> CommandLineState {
        link.status().state
    }

    #[test]
    fn status_reads_each_state_of_the_link() {
        let root = TempDir::new().unwrap_or_else(|_| panic!("temporary folder"));
        let link = link(root.path());
        assert_eq!(state(&link), CommandLineState::NotInstalled);

        let ours = link.command.clone().unwrap_or_else(|| panic!("command"));
        symlink(&ours, &link.link).unwrap_or_else(|_| panic!("link"));
        assert_eq!(state(&link), CommandLineState::Installed);

        fs::remove_file(&link.link).unwrap_or_else(|_| panic!("unlink"));
        let moved = root
            .path()
            .join("Downloads/SkillBinder.app/Contents/MacOS/skillbinder-cli");
        symlink(&moved, &link.link).unwrap_or_else(|_| panic!("link"));
        assert_eq!(state(&link), CommandLineState::OtherApp);

        fs::remove_file(&link.link).unwrap_or_else(|_| panic!("unlink"));
        symlink(root.path().join("elsewhere"), &link.link).unwrap_or_else(|_| panic!("link"));
        assert_eq!(state(&link), CommandLineState::InTheWay);

        fs::remove_file(&link.link).unwrap_or_else(|_| panic!("unlink"));
        write(&link.link, "someone else's");
        assert_eq!(state(&link), CommandLineState::InTheWay);
    }

    #[test]
    fn outside_applications_the_command_cant_be_installed() {
        let root = TempDir::new().unwrap_or_else(|_| panic!("temporary folder"));
        let link = Link {
            command: None,
            ..link(root.path())
        };
        assert_eq!(
            state(&link),
            CommandLineState::CannotInstall(CannotInstallCommand::OutsideApplications)
        );
        // A file in the way is still named, since moving the app wouldn't help.
        write(&link.link, "someone else's");
        assert_eq!(state(&link), CommandLineState::InTheWay);
    }

    #[test]
    fn homebrews_link_is_known_by_the_cask_under_each_prefix() {
        for (prefix, cask) in [
            ("opt/homebrew", "skillbinder"),
            ("usr/local", "skillbinder@beta"),
        ] {
            let root = TempDir::new().unwrap_or_else(|_| panic!("temporary folder"));
            let link = link(root.path());
            let prefix = root.path().join(prefix);
            fs::create_dir_all(prefix.join("Caskroom").join(cask))
                .unwrap_or_else(|_| panic!("caskroom"));
            let brewed = prefix.join("bin/skillbinder");
            fs::create_dir_all(prefix.join("bin")).unwrap_or_else(|_| panic!("bin"));
            let command = link.command.clone().unwrap_or_else(|| panic!("command"));
            symlink(command, &brewed).unwrap_or_else(|_| panic!("link"));
            let status = link.status();
            assert_eq!(status.state, CommandLineState::Homebrew, "{cask}");
            assert_eq!(status.path, brewed);
        }
    }

    #[test]
    fn install_repairs_and_uninstall_removes_only_the_link() {
        let root = TempDir::new().unwrap_or_else(|_| panic!("temporary folder"));
        let link = link(root.path());
        let path = link.link.clone();
        let moved = root
            .path()
            .join("Downloads/SkillBinder.app/Contents/MacOS/skillbinder-cli");
        symlink(&moved, &path).unwrap_or_else(|_| panic!("link"));
        let command_line = command_line(root.path(), Place::Link(link));

        let repaired = command_line
            .install()
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(repaired.state, CommandLineState::Installed);

        let removed = command_line
            .uninstall()
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(removed.state, CommandLineState::NotInstalled);
        assert!(fs::symlink_metadata(&path).is_err());
    }

    #[test]
    fn a_skillbinder_it_didnt_write_is_never_replaced() {
        let root = TempDir::new().unwrap_or_else(|_| panic!("temporary folder"));
        let link = link(root.path());
        let path = link.link.clone();
        write(&path, "someone else's");
        let command_line = command_line(root.path(), Place::Link(link));
        assert!(command_line.install().is_err());
        assert!(command_line.uninstall().is_err());
        assert_eq!(
            fs::read_to_string(&path).unwrap_or_default(),
            "someone else's"
        );
    }
}

mod copy {
    use super::*;

    fn copy(root: &Path) -> Copy {
        let command = root.join("appimage/usr/bin/skillbinder-cli");
        write(&command, "version 1");
        Copy {
            command: Some(command),
            copy: root.join("home/.local/bin/skillbinder"),
            record: root.join("data/command-line.sha256"),
            on_path: true,
        }
    }

    fn installed(root: &Path, copy: Copy) -> (CommandLine, std::path::PathBuf) {
        fs::create_dir_all(root.join("data")).unwrap_or_else(|_| panic!("data"));
        let command = copy.command.clone().unwrap_or_else(|| panic!("command"));
        let command_line = command_line(root, Place::Copy(copy));
        let status = command_line
            .install()
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(status.state, CommandLineState::Installed);
        (command_line, command)
    }

    #[test]
    fn status_reads_each_state_of_the_copy() {
        let root = TempDir::new().unwrap_or_else(|_| panic!("temporary folder"));
        assert_eq!(
            copy(root.path()).status().state,
            CommandLineState::NotInstalled
        );
        let not_appimage = Copy {
            command: None,
            ..copy(root.path())
        };
        assert_eq!(
            not_appimage.status().state,
            CommandLineState::CannotInstall(CannotInstallCommand::NotAppImage)
        );
        let off_path = Copy {
            on_path: false,
            ..copy(root.path())
        };
        let (command_line, _) = installed(root.path(), copy(root.path()));
        assert_eq!(off_path.status().state, CommandLineState::NotOnPath);
        assert_eq!(command_line.status().state, CommandLineState::Installed);
    }

    #[test]
    fn a_copy_with_an_unrecorded_digest_is_in_the_way() {
        let root = TempDir::new().unwrap_or_else(|_| panic!("temporary folder"));
        let copy = copy(root.path());
        // The same bytes as the command, but SkillBinder didn't write them.
        write(&copy.copy, "version 1");
        assert_eq!(copy.status().state, CommandLineState::InTheWay);
    }

    #[test]
    fn launch_follows_the_running_appimage_and_leaves_an_edited_copy() {
        let root = TempDir::new().unwrap_or_else(|_| panic!("temporary folder"));
        let (command_line, command) = installed(root.path(), copy(root.path()));
        let path = command_line.status().path;

        write(&command, "version 2");
        assert_eq!(command_line.status().state, CommandLineState::OtherApp);
        command_line.refresh();
        assert_eq!(command_line.status().state, CommandLineState::Installed);
        assert_eq!(fs::read_to_string(&path).unwrap_or_default(), "version 2");

        write(&path, "edited");
        write(&command, "version 3");
        command_line.refresh();
        assert_eq!(command_line.status().state, CommandLineState::InTheWay);
        assert_eq!(fs::read_to_string(&path).unwrap_or_default(), "edited");
    }

    #[test]
    fn uninstall_removes_the_copy_and_its_record() {
        let root = TempDir::new().unwrap_or_else(|_| panic!("temporary folder"));
        let (command_line, _) = installed(root.path(), copy(root.path()));
        let status = command_line
            .uninstall()
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(status.state, CommandLineState::NotInstalled);
        assert!(!root.path().join("data/command-line.sha256").exists());
    }

    #[cfg(unix)]
    #[test]
    fn the_copy_is_executable() {
        use std::os::unix::fs::PermissionsExt;
        let root = TempDir::new().unwrap_or_else(|_| panic!("temporary folder"));
        let (command_line, _) = installed(root.path(), copy(root.path()));
        let mode = fs::metadata(command_line.status().path)
            .map(|metadata| metadata.permissions().mode())
            .unwrap_or_default();
        assert_eq!(mode & 0o777, 0o755);
    }
}

#[test]
fn the_bin_folder_is_added_to_the_users_path_once_and_removed() {
    let bin = r"C:\Users\me\AppData\Local\Programs\SkillBinder\bin";
    assert_eq!(with("", bin), bin);
    let path = with(r"%USERPROFILE%\.cargo\bin;", bin);
    assert_eq!(path, format!(r"%USERPROFILE%\.cargo\bin;{bin}"));
    assert!(contains(&path, bin));
    assert!(contains(&path, &format!("{}\\", bin.to_uppercase())));
    assert_eq!(without(&path, bin), r"%USERPROFILE%\.cargo\bin");
    assert_eq!(without(r"C:\a;C:\b", bin), r"C:\a;C:\b");
}
