//! Installs an app update at quit (spec rule 12, ADR 0018, ADR 0019): on macOS by renames on one
//! volume, so it is quick and can be undone; on Windows by starting the setup program silently,
//! which waits for the app to exit.

use std::{
    fs, io,
    path::Path,
    process::{Command, Stdio},
};

/// Puts `staged` at `target`. A bundle already at `target` is renamed to `old` first, and put back
/// when `staged` can't take its place. The caller removes `old`.
pub(super) fn swap(staged: &Path, target: &Path, old: &Path) -> io::Result<()> {
    if fs::symlink_metadata(target).is_err() {
        return fs::rename(staged, target);
    }
    let _ = fs::remove_dir_all(old);
    fs::rename(target, old)?;
    if let Err(error) = fs::rename(staged, target) {
        let _ = fs::rename(old, target);
        return Err(error);
    }
    Ok(())
}

/// Starts the setup program `setup` silently and detached, so it outlives the app.
pub(super) fn start_setup(setup: &Path, pid: u32, relaunch: bool) -> io::Result<()> {
    let mut command = Command::new(setup);
    command
        .args(setup_arguments(pid, relaunch))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }
    command.spawn().map(drop)
}

/// The setup program's command line (`packaging/windows/skillbinder.iss`): silent, without
/// message boxes or a reboot, waiting for the app, process `pid`, to exit, and opening the new
/// version only after Restart.
fn setup_arguments(pid: u32, relaunch: bool) -> Vec<String> {
    let mut arguments = ["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART"]
        .map(str::to_owned)
        .to_vec();
    arguments.push(format!("/WAITPID={pid}"));
    if relaunch {
        arguments.push("/RELAUNCH=1".to_owned());
    }
    arguments
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "Test fixture failures must abort the test."
    )]
    use super::*;

    fn bundle(path: &Path, marker: &str) {
        fs::create_dir_all(path.join("Contents")).unwrap();
        fs::write(path.join("Contents/marker"), marker).unwrap();
    }

    fn marker(path: &Path) -> String {
        fs::read_to_string(path.join("Contents/marker")).unwrap()
    }

    #[test]
    fn the_staged_bundle_takes_the_installed_ones_place() {
        let temp = tempfile::tempdir().unwrap();
        let (staged, target, old) = (
            temp.path().join(".skillbinder-update-0.2.0.app"),
            temp.path().join("SkillBinder.app"),
            temp.path().join(".skillbinder-old-0.1.0.app"),
        );
        bundle(&staged, "new");
        bundle(&target, "running");
        swap(&staged, &target, &old).unwrap();
        assert_eq!(marker(&target), "new");
        assert_eq!(marker(&old), "running");
        assert!(!staged.exists());
    }

    #[test]
    fn the_silent_setup_waits_for_the_app_and_relaunches_only_after_restart() {
        let silent = [
            "/VERYSILENT",
            "/SUPPRESSMSGBOXES",
            "/NORESTART",
            "/WAITPID=4242",
        ];
        assert_eq!(setup_arguments(4242, false), silent);
        assert_eq!(
            setup_arguments(4242, true),
            [silent.as_slice(), &["/RELAUNCH=1"]].concat()
        );
    }

    #[test]
    fn a_failed_second_rename_restores_the_installed_bundle() {
        let temp = tempfile::tempdir().unwrap();
        let (target, old) = (
            temp.path().join("SkillBinder.app"),
            temp.path().join(".skillbinder-old-0.1.0.app"),
        );
        bundle(&target, "running");
        let missing = temp.path().join(".skillbinder-update-0.2.0.app");
        assert!(swap(&missing, &target, &old).is_err());
        assert_eq!(marker(&target), "running");
        assert!(!old.exists());
    }
}
