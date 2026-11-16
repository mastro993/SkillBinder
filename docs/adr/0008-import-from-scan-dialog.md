# Import happens in one dialog, from onboarding or Scan for skills; Discovery is retired

Importing found skills is something a user does when the library starts and, now and then, after new skills appear in their agents or workspaces. A whole page in the sidebar for it was more than it earns. Discovery, the view of every found skill that is not in library, is removed with its sidebar row. Import happens in exactly two places, which show the same Import skills dialog: onboarding's last step, over your agents' global skill directories, and Library's Scan for skills button, which runs a fresh scan of the global skill directories and every workspace and then opens it. Found skills that are not in library are listed only in that dialog. This partly supersedes ADR 0007, which listed them only in Discovery.

The dialog joins Discovery's selection and the import review into one step. It lists each found skill not in library once, with its versions, validation, and the copies importing it replaces, and every importable skill whose chosen version is valid starts ticked, up to the 500 one import takes. Unticking a skill, choosing another version, or acknowledging invalid metadata happens in the same list, and Import applies what is ticked. Seeing the replacements before Import is still what confirms them (ADR 0006).

Scans keep running in the background on launch, on focus, and after every change that writes copies, because each scan request first retries the copies that are not applied yet and each completed scan lets library skills learn their source. Only importing moved. The Scan for skills button shows how many found skills are not in library, so skills that appear in a newly added workspace are noticed without a page for them. A library with no skills shows an empty state with the same button; nothing opens by itself.

## Considered Options

- Keeping Discovery as a page and only adding the Library button was rejected. It keeps two entry points to one act and a sidebar row that is empty for most users most of the time.
- Scanning only at onboarding and on Scan for skills was rejected. Copies that failed to write would no longer be retried, sources would no longer be learned, and the found skills behind the count would go stale.
- Keeping selection and review as two dialogs was rejected. The review only repeats the selection with the replacements added, and onboarding already shows the review rows as its list.
- Starting with nothing ticked was rejected. Importing everything found is the common case, and onboarding already starts with every skill it offers ticked. Skills the user skips come back ticked on the next Scan for skills; a remembered skip list was rejected as machinery nobody asked for.
- Opening the dialog by itself whenever the library is empty was rejected. It would interrupt a user who keeps the library empty on purpose, such as before a first Pull.

## Consequences

- The routes that fell back to Discovery, a removed workspace's page and the page of an agent no longer detected, open Library.
- The persistent "Import finished" result is removed; the progress strip's end confirms an import.
- Onboarding now also offers skills it would make active nowhere, such as one found only through a link; they import inactive, as Discovery imported them.
