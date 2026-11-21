//! Add to the applications menu (spec rule 23, ADR 0019): writes `skillbinder.desktop` and its
//! icon under the user's data home, pointing at the AppImage's path. An app update replaces the
//! file at that path, so the entry keeps opening the installed version.

use std::{fs, io, path::Path};

/// The icon `xtask package` puts in every archive.
const ICON: &[u8] = include_bytes!("../../../../assets/icon.png");
/// Where the icon goes under the data home: the size `ICON` is, in the theme every desktop reads.
const ICON_PATH: &str = "icons/hicolor/32x32/apps/skillbinder.png";
const ENTRY_PATH: &str = "applications/skillbinder.desktop";

/// Writes the menu entry starting `appimage`, and its icon, under `data_home`, replacing what an
/// earlier Add wrote. The entry is written last, so the menu never shows it without its icon.
pub(super) fn write(appimage: &Path, data_home: &Path) -> io::Result<()> {
    let entry = entry(appimage)?;
    replace(&data_home.join(ICON_PATH), ICON)?;
    replace(&data_home.join(ENTRY_PATH), entry.as_bytes())
}

/// Writes `contents` beside `path`, then renames it into place, so a desktop watching the folder
/// never reads half a file.
fn replace(path: &Path, contents: &[u8]) -> io::Result<()> {
    let folder = path.parent().ok_or(io::ErrorKind::InvalidInput)?;
    fs::create_dir_all(folder)?;
    let partial = path.with_extension(format!("partial-{}", std::process::id()));
    fs::write(&partial, contents)
        .and_then(|()| fs::rename(&partial, path))
        .inspect_err(|_| {
            let _ = fs::remove_file(&partial);
        })
}

/// The desktop entry. `StartupWMClass` matches the window's app id, so the dock groups the
/// window under the entry.
fn entry(appimage: &Path) -> io::Result<String> {
    let exec = quoted(appimage.to_str().ok_or(io::ErrorKind::InvalidInput)?)?;
    Ok(format!(
        "[Desktop Entry]\nType=Application\nName=SkillBinder\nComment=Manage a local library of agent skills\nExec={exec}\nIcon=skillbinder\nStartupWMClass=skillbinder\nTerminal=false\nCategories=Development;Utility;\n"
    ))
}

/// `path` as one quoted `Exec` argument (Desktop Entry Specification, "The Exec key"): `"`,
/// `` ` ``, `$`, and `\` are escaped inside the quotes, then each backslash again as the string
/// value's own escape, and `%` is doubled so it is no field code. A control character can't be
/// written on the entry's one line.
fn quoted(path: &str) -> io::Result<String> {
    if path.chars().any(char::is_control) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let mut exec = String::from("\"");
    for character in path.chars() {
        match character {
            '"' | '`' | '$' => exec.extend(['\\', '\\', character]),
            '\\' => exec.push_str(r"\\\\"),
            '%' => exec.push_str("%%"),
            other => exec.push(other),
        }
    }
    exec.push('"');
    Ok(exec)
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unwrap_used,
        reason = "Test fixture failures must abort the test."
    )]
    use super::*;

    #[test]
    fn the_entry_and_icon_point_at_the_appimage() {
        let home = tempfile::tempdir().unwrap();
        let data_home = home.path().join(".local/share");
        let appimage = home
            .path()
            .join("Applications/skillbinder-0.1.0-x86_64.AppImage");
        write(&appimage, &data_home).unwrap();
        let entry = fs::read_to_string(data_home.join(ENTRY_PATH)).unwrap();
        let exec = quoted(appimage.to_str().unwrap()).unwrap();
        assert!(entry.contains(&format!("\nExec={exec}\n")), "{entry}");
        assert!(entry.contains("\nIcon=skillbinder\n"), "{entry}");
        assert_eq!(fs::read(data_home.join(ICON_PATH)).unwrap(), ICON);
        // Adding again replaces both, leaving nothing beside them.
        write(&appimage, &data_home).unwrap();
        let entries = fs::read_dir(data_home.join("applications"))
            .unwrap()
            .count();
        assert_eq!(entries, 1);
    }

    #[test]
    fn the_path_is_one_quoted_exec_argument() {
        assert_eq!(
            quoted("/home/me/My Apps/skillbinder.AppImage").unwrap(),
            r#""/home/me/My Apps/skillbinder.AppImage""#
        );
        assert_eq!(
            quoted(r#"/a "b" `c` $d \e 100%"#).unwrap(),
            r#""/a \\"b\\" \\`c\\` \\$d \\\\e 100%%""#
        );
        assert!(quoted("/home/me/new\nline").is_err());
    }
}
