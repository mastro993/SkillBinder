# Library organization

Library skills can have one folder and multiple tags. Folders are a single flat level; they cannot contain other folders. Folder records, tag records, and each skill's folder and tag assignment live in the portable `.skillbinder.json` metadata, and SQLite does not own this data. Tags are stored and validated but have no user interface yet. Changing organization does not move skill payloads or deployed copies.

## Folders

The Library screen lists every skill by default and creates folders from its top-right **Add folder** action, which asks for a name only.

A folder opens its own page at `/library/<folder>` and shows the skills assigned to it. Discovery and Skills appear together in the sidebar, followed by a separate **Folders** group. Its plus button opens the same new-folder dialog as the Library screen, even when no folders exist. Folder links are alphabetical, with folder icons, a deeper left inset, and no vertical guide or right inset. Badges show the total skill count beside Skills and the assigned skill count beside each folder, including zero. The selected folder is highlighted; Skills is highlighted only on the all-skills page. Long labels are ellipsized without horizontal scrolling, with full names available on hover. The folder group scrolls independently of the pinned logo, Sync, and Settings links. On desktop, drag the sidebar's right edge to resize it; the width is saved across sessions. The edge is keyboard accessible with Left/Right Arrow, Home, and End. In narrow windows, the menu button opens the complete navigation in a drawer. That page's **Edit folder** action renames or deletes the folder. Deleting asks for confirmation with the number of skills that will move to Unfiled, then returns to the Library screen. Skills keep at most one folder: assigning a skill to a new folder replaces the previous one.

## Library screen

The Library screen shows all skills, filtered by search alone. A result with no matches is distinct from a library with no imported skills. The card action organizes one skill; selecting skill cards offers **Organize selected**, which assigns every selected skill to one folder or leaves their folders alone. Both open the same folder picker.

Tags are part of the stored model and the IPC contract, but no screen exposes them yet.

A delete preview counts the skills assigned to a folder. Deleting a folder moves its skills to Unfiled. The confirmation carries a metadata revision and fails if the organization changed after preview.

## Validation and durability

Core validates the complete graph after each proposed change. Folder names and tag names are unique, compared with Unicode compatibility normalization and full case folding. IDs must be safe path components. Missing references and unsorted or repeated tag IDs are refused. Rust generates IDs for new folders and tags.

A record read from `.skillbinder.json` must not brick the library when it carries a field this version no longer writes, so unknown folder and tag fields are ignored and dropped at the next write.

Platform serializes every writer of `.skillbinder.json` with one lock, because an import that appends a skill and an organization change both read the file, edit it, and write it back. Each write goes to a sibling temporary file that is renamed over the target, so an interrupted write leaves the previous metadata intact. A change that arrives with a metadata revision is refused when the organization changed since that revision.
Organization changes leave Git working tree changes for the user to commit explicitly, as imports do.

`library_list` returns skills, folders, tags, and the organization revision in one response; the screens ignore tags for now. `library_organization_change` applies one validated change, and folder assignment always sends empty tag lists; `library_organization_preview_delete` returns the counts and revision needed for confirmation. The frontend validates both request and response shapes with Zod.
