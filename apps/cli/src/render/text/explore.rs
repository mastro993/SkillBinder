//! `scan`, `search`, and `repos` as text.

use skillbinder_proto::ValidationStatus;
use skillbinder_proto::command_line::{
    FoundOutput, FoundRow, FoundVersion, RepositoryRow, SearchOutput, SearchRow, SkillsShAnswer,
};

/// The found skills, then every ignored slug when any is.
pub(super) fn found(output: &FoundOutput) -> String {
    let mut text = offered(&output.found);
    if !output.ignored.is_empty() {
        text.push_str(&format!("Ignored: {}\n", output.ignored.join(", ")));
    }
    text
}

/// Each found skill under its group, with its versions, then how to import them.
fn offered(rows: &[FoundRow]) -> String {
    if rows.is_empty() {
        return "Found no skills that aren't in your library.\n".to_owned();
    }
    let mut text = String::new();
    let mut place = None;
    for row in rows {
        if place != Some(&row.place) {
            place = Some(&row.place);
            text.push_str(&format!("{}:\n", row.place));
        }
        text.push_str(&found_row(row));
    }
    if rows.iter().any(|row| row.versions.len() > 1) {
        text.push_str("* is the version an import takes; name another as <slug>@<version>.\n");
    }
    let ticked = rows.iter().filter(|row| row.ticked).count();
    text.push_str(&format!(
        "Import with `skillbinder import <slug>`, or `skillbinder import --all` for {ticked} of \
         them.\n"
    ));
    text
}

/// `  pdf: Fill PDFs.`, then a line per version: its short digest, `*` on the one an import
/// takes among several, its status, and where it was found; then the copies it replaces.
fn found_row(row: &FoundRow) -> String {
    let description = match row.description.as_str() {
        "" => String::new(),
        description => format!(": {description}"),
    };
    let mut text = format!("  {}{description}\n", row.slug);
    let several = row.versions.len() > 1;
    for version in &row.versions {
        text.push_str(&version_line(version, several));
        for place in &version.replaces {
            text.push_str(&format!("      Replaces the copy in {place}.\n"));
        }
    }
    text
}

fn version_line(version: &FoundVersion, several: bool) -> String {
    let mark = if several && version.planned { "*" } else { " " };
    let status = match version.validation {
        ValidationStatus::Valid | ValidationStatus::Warning => "",
        ValidationStatus::Invalid => " [invalid]",
        ValidationStatus::Blocked => " [blocked]",
    };
    format!(
        "    {}{mark} {}{status}\n",
        version.version,
        version.locations.join("; ")
    )
}

/// Each result with what `install` takes for it, then why skills.sh's are missing.
pub(super) fn search(output: &SearchOutput) -> String {
    let mut text: String = output.results.iter().map(search_row).collect();
    if output.results.is_empty() {
        text.push_str("No skills found.\n");
    }
    match output.skills_sh {
        SkillsShAnswer::Answered | SkillsShAnswer::NotAsked => {}
        SkillsShAnswer::Offline => text.push_str("skills.sh couldn't be reached.\n"),
        SkillsShAnswer::Unavailable => text.push_str("skills.sh isn't available right now.\n"),
    }
    text
}

/// `anthropics/skills/pdf (Anthropic, +900 this week) [in library]`, then its description.
fn search_row(row: &SearchRow) -> String {
    let about = [Some(row.source.as_str()), row.installs.as_deref()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", ");
    let mark = if row.in_library {
        " [in library]"
    } else if row.collides {
        " [another skill in your library has its name]"
    } else {
        ""
    };
    let mut text = format!("{} ({about}){mark}\n", row.reference);
    if !row.description.is_empty() {
        text.push_str(&format!("  {}\n", row.description));
    }
    text
}

/// `acme/skills (14 skills, 3 not in library): https://github.com/acme/skills`.
pub(super) fn repositories(rows: &[RepositoryRow]) -> String {
    if rows.is_empty() {
        return "No repositories. Add one with `skillbinder repo add <url>`.\n".to_owned();
    }
    rows.iter()
        .map(|row| {
            format!(
                "{} ({}, {} not in library): {}\n",
                row.name,
                super::count(row.skills, "skill"),
                row.installable,
                row.url
            )
        })
        .collect()
}
