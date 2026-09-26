# Library organization

Library skills can have one folder and multiple tags. Folders are a single flat level; they cannot contain other folders. Folder records, tag records, and each skill's folder and tag assignment live in the portable `.skillbinder.json` metadata, and SQLite does not own this data. Changing organization does not move skill payloads or deployed copies.

## Folders

The Library screen lists every skill by default and creates folders from its top-right **Add folder** action, which asks for a name only.

A folder opens its own page at `/library/<folder>` and shows the skills assigned to it. The sidebar lists the folders that exist, directly below the Library item, and each one links to its page; the sidebar has no create or edit controls. That page's **Edit folder** action renames or deletes the folder. Deleting asks for confirmation with the number of skills that will move to Unfiled, then returns to the Library screen. Skills keep at most one folder: assigning a skill to a new folder replaces the previous one.

## Library screen

The Library screen shows all skills, with tags as the only filter column. Multiple selected tags match all by default; the user can switch to match any. Search combines with the tag filter. Clear filters restores the full list. A result with no matches is distinct from a library with no imported skills.

The screen can create, rename, and delete tags. The card action edits one skill; selection supports assigning a folder and adding or removing tags from several skills. Bulk tag edits leave each skill's other tags in place. A bulk tag edit leaves folders unchanged when “Keep current folders” is selected.

A delete preview counts the skills assigned to a folder, or the skills carrying a tag. Deleting a folder moves its skills to Unfiled. Deleting a tag removes that tag from every skill. The confirmation carries a metadata revision and fails if the organization changed after preview.

## Validation and durability

Core validates the complete graph after each proposed change. Folder names and tag names are unique, compared with Unicode compatibility normalization and full case folding. IDs must be safe path components. Missing references and unsorted or repeated tag IDs are refused. Rust generates IDs for new folders and tags.

Platform serializes every writer of `.skillbinder.json` with one lock, because an import that appends a skill and an organization change both read the file, edit it, and write it back. Each write goes to a sibling temporary file that is renamed over the target, so an interrupted write leaves the previous metadata intact. A change that arrives with a metadata revision is refused when the organization changed since that revision.
Organization changes leave Git working tree changes for the user to commit explicitly, as imports do.

`library_list` returns skills, folders, tags, and the organization revision in one response. `library_organization_change` applies one validated change; `library_organization_preview_delete` returns the counts and revision needed for confirmation. The frontend validates both request and response shapes with Zod.
