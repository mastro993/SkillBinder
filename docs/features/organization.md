# Library organization

Library skills can have one folder and multiple tags. Folders can contain other folders. Folder records, tag records, and each skill's folder and tag assignment live in the portable `.skillbinder.json` metadata, and SQLite does not own this data. Changing organization does not move skill payloads or deployed copies.

## Library screen

The Library screen lists all skills by default. The sidebar filters to one folder and its descendants, Unfiled, or selected tags. Multiple selected tags match all by default; the user can switch to match any. Search combines with those filters. Clear filters restores the full list. A result with no matches is distinct from a library with no imported skills.

The screen can create, rename, move, and delete folders and tags. The card action edits one skill; selection supports assigning a folder and adding or removing tags from several skills. Bulk tag edits leave each skill's other tags in place. A bulk tag edit leaves folders unchanged when “Keep current folders” is selected.

A delete preview counts direct child folders and directly assigned skills for a folder, or assigned skills for a tag. Deleting a folder reparents its direct children and skills to its parent. Deleting a tag removes that tag from every skill. A folder delete is refused if reparenting would duplicate a sibling name. The confirmation carries a metadata revision and fails if the organization changed after preview.

## Validation and durability

Core validates the complete graph after each proposed change. Folder names are unique among siblings; tag names are unique globally. Comparison uses Unicode compatibility normalization and full case folding. IDs must be safe path components. Parent cycles, missing references, and unsorted or repeated tag IDs are refused. Rust generates IDs for new folders and tags.

Platform serializes every writer of `.skillbinder.json` with one lock, because an import that appends a skill and an organization change both read the file, edit it, and write it back. Each write goes to a sibling temporary file that is renamed over the target, so an interrupted write leaves the previous metadata intact. A change that arrives with a metadata revision is refused when the organization changed since that revision.
Organization changes leave Git working tree changes for the user to commit explicitly, as imports do.

`library_list` returns skills, folders, tags, and the organization revision in one response. `library_organization_change` applies one validated change; `library_organization_preview_delete` returns the counts and revision needed for confirmation. The frontend validates both request and response shapes with Zod.
