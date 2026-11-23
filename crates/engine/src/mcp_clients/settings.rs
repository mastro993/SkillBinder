//! Where the agents keep their settings and programs, and reading, never writing, whether a
//! settings file names the server. VS Code's `mcp.json` is JSON with comments and trailing commas.

use super::Places;
use std::path::{Path, PathBuf};

/// VS Code's user settings folder: under Application Support on macOS, `%APPDATA%` on Windows,
/// and the XDG config folder elsewhere.
pub(super) fn vs_code_user(places: &Places) -> PathBuf {
    let base = if cfg!(target_os = "macos") {
        places.home.join("Library/Application Support")
    } else if cfg!(windows) {
        places
            .variable("APPDATA")
            .unwrap_or_else(|| places.home.join("AppData").join("Roaming"))
    } else {
        places
            .variable("XDG_CONFIG_HOME")
            .unwrap_or_else(|| places.home.join(".config"))
    };
    base.join("Code").join("User")
}

/// VS Code's user `mcp.json`, then each profile's.
pub(super) fn vs_code_files(places: &Places) -> Vec<PathBuf> {
    let user = vs_code_user(places);
    let profiles = std::fs::read_dir(user.join("profiles"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|profile| profile.path().join("mcp.json"));
    std::iter::once(user.join("mcp.json"))
        .chain(profiles)
        .collect()
}

/// Where the agents' programs are installed besides the PATH: the native installers' and npm's
/// folders, and VS Code's own `bin`, whose `code` the user may never have added to the PATH.
pub(super) fn install_folders(places: &Places) -> Vec<PathBuf> {
    let home = &places.home;
    let mut folders = vec![home.join(".local").join("bin")];
    if cfg!(windows) {
        folders.extend(places.variable("APPDATA").map(|data| data.join("npm")));
        folders.extend(
            places
                .variable("LOCALAPPDATA")
                .map(|data| data.join("Programs").join("Microsoft VS Code").join("bin")),
        );
    } else {
        folders.push(home.join(".claude").join("local"));
    }
    if cfg!(target_os = "macos") {
        let bundle = "Visual Studio Code.app/Contents/Resources/app/bin";
        folders.push(Path::new("/Applications").join(bundle));
        folders.push(home.join("Applications").join(bundle));
    }
    folders
}

/// Whether any of `files` holds `name` in its `key` object. A file missing or unreadable names
/// nothing.
pub(super) fn names_server(files: &[PathBuf], key: &str, name: &str) -> bool {
    files.iter().any(|file| {
        std::fs::read_to_string(file)
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&plain_json(&text)).ok())
            .is_some_and(|settings| {
                settings
                    .get(key)
                    .and_then(|servers| servers.get(name))
                    .is_some()
            })
    })
}

/// `text` without comments and trailing commas, which VS Code's settings allow.
pub(super) fn plain_json(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut string = false;
    // Where the last comma outside a string went, until something other than space follows it.
    let mut comma: Option<usize> = None;
    while let Some(char) = chars.next() {
        if string {
            plain.push(char);
            match char {
                '\\' => plain.extend(chars.next()),
                '"' => string = false,
                _ => {}
            }
            continue;
        }
        match (char, chars.peek()) {
            ('/', Some('/')) => while chars.next_if(|next| *next != '\n').is_some() {},
            ('/', Some('*')) => {
                chars.next();
                let mut last = None;
                for next in chars.by_ref() {
                    if last == Some('*') && next == '/' {
                        break;
                    }
                    last = Some(next);
                }
            }
            (char, _) if char.is_whitespace() => plain.push(char),
            (char, _) => {
                if let Some(at) = comma.take()
                    && matches!(char, '}' | ']')
                {
                    plain.remove(at);
                }
                if char == ',' {
                    comma = Some(plain.len());
                }
                string = char == '"';
                plain.push(char);
            }
        }
    }
    plain
}
