# Native feature coverage

The audited shipped baseline is revision `83d55ef789a7cf256021f6620742e0e9021a8312`. Main was fetched and the rewrite branch rebased before reconstruction; that revision remained current. The baseline exposes 29 application operations across the journeys below. The user deferred UI fidelity and polish; functional behavior remains required.

## Setup and lifecycle

Four persisted steps cover prerequisites, source boundaries, optional synchronization, and final library creation. Backward navigation is allowed; forward skipping is refused. Git 2.39 or newer and writable storage are required. Library initialization stages metadata and a repository before atomically publishing the directory. Completion is idempotent.

The executable owns the Tokio runtime and process lock. A second launch requests activation of the existing native window through an app-owned local marker. The Linux window uses the `skillbinder` application ID matching its desktop entry; Wayland compositors retain control over whether an activation request raises the window or requests attention. Shutdown cancels discovery and drains durable operations before flushing logs.

## Application shell

On macOS the window uses a transparent titlebar, so the shell fills the whole window and the traffic lights float over its top-left corner. Windows and Linux keep native decorations. The shell has a sidebar, a header, and a status bar along the bottom of the window.

The sidebar has a one-device-pixel right border, which is also the resize divider, and a background slightly darker than the main surface. Users can resize it by dragging its edge or with the arrow, Home, and End keys on its resize control. Width is clamped to 192 through 400 logical pixels. It can be hidden from the sidebar toggle, the View menu, or Cmd+B on macOS and Ctrl+B on Windows and Linux. A hidden sidebar is removed entirely, with no icon rail. Width and hidden state are machine-local preferences that persist across restarts. Preferences stored before the hidden state existed load as visible.

The sidebar toggle stays at one position whether the sidebar is open or closed. On macOS it sits just right of the traffic lights, and elsewhere it sits at the leading edge of the header row. Opening and closing take 220 ms with cubic ease-in-out. The sidebar's footprint widens or narrows, so the main area reflows. Meanwhile the full-width sidebar stays pinned to the moving edge and slides in or out. The header's leading inset follows the same curve to stay clear of the toggle. A closing sidebar unmounts after its transition so its controls leave keyboard order. The launch state appears without motion. When the app reports reduced motion, the change happens immediately.

The header starts at the sidebar's right edge, is 50 points tall, and has no border or background. On macOS the traffic lights sit 16 points from the left edge, vertically centred in that row, and the toggle follows one light-spacing after them. Its trailing group holds buttons that the active screen supplies; Discovery supplies its scan controls there. On macOS the header doubles as the window drag region. Onboarding shows the header without the toggle or screen buttons.

The status bar spans the full window width below the sidebar and main area. It is 30 points tall, shares the sidebar's background and border color, and has a one-device-pixel top border. Items sit in left, center, and right slots, 12 points in from the window edges. Neighbouring items in a slot are split by 12-point separators, inset from the bar's top and bottom, with 12 points of space on each side. The left slot holds a gear that opens the Settings screen, an appearance toggle, the application version, and the Git status items. The version shows a box icon, 8 points apart from the bare version number, such as `0.1.0`. Its tooltip reads Check for updates; the version is not clickable yet. The gear and toggle are icon-only, with tooltips and accessibility labels, and sit 8 points apart with no separator between them; the version and Contribute pair an icon with a label. Icon-only items use 14-point icons in 24-point square buttons; icon-and-text items use 12-point icons. Item text and icons share the muted text color, including on hover, where buttons show only a background. The right slot holds Contribute, which opens the GitHub repository in the default browser. The center slot is empty. Onboarding has no status bar.

Status bar tooltips invert the theme: dark with light text on the light theme, light with dark text on the dark one. They open above their item after half a second, shift sideways to stay inside the window, and close as soon as the pointer leaves. GPUI Kit tooltips always use the popover colors, so the status bar builds them from GPUI Kit hover cards.

The Git status items are, in order: the branch, Commit, the sync status, and History. Each pairs an icon with a label and has a tooltip: Current branch, Commit and push changes, Synchronize now, and Open changes history. The branch shows the configured remote branch, or `main`, the managed library's initial branch, when no remote is connected. Commit shows a rounded badge with the number of managed paths that have uncommitted changes; the badge is hidden when there are none. A rename counts once. The sync status reads Synced when the library matches the remote and Not synced otherwise, including when no remote is connected. While a Git operation runs, including focus and Sync screen refreshes, it reads Synchronizing and its icon spins. Clicking these items does nothing yet.

The appearance toggle shows the current theme, a sun for light and a moon for dark, and switches to the other one. Its tooltip names the theme it switches to. The toggle has two visible states but stores one of three preferences: System, Light, or Dark. A press that switches away from the system appearance stores that theme as an override. A press that switches back to the system appearance clears the override, so the app follows the system again. The override is checked only when the toggle is pressed. If the system appearance later changes to match an override, the override is kept, so the app stays on that theme when the system changes again. The preference is machine-local and persists across restarts. Preferences stored before the choice existed follow the system appearance.

Every button, including the sidebar toggle and dialog close buttons, shows a hand cursor while enabled and the arrow while disabled. GPUI Kit buttons keep the arrow except link and text variants, so the app builds its own: dialogs replace the kit's close control with an app button in the title row. That button is hidden while an operation is pending, as the kit control was.

## Discovery and roots

The retained registry covers 79 agents. Templates expand home and environment defaults. Shared physical skill directories list every reader label. Project registrations search only known direct skill directories beneath the selected canonical boundary, not arbitrary document folders. Global and project traversal have separate depth, entry, exclusion, and mount policies. Links are limited to 16 hops, including links inside registered discovery roots before physical-path deduplication. Source aliases retain their original path for import revalidation.

Settings uses native directory selection to issue a single-use grant valid for 300 seconds. Roots can be labeled, renamed, disabled, enabled, and removed. Duplicate canonical registrations name the existing root. Removing a registration never removes its files.

Discovery supports start, current results, cancellation, and rescan. There is at most one active worker. Cancelled and failed findings remain visible but cannot be imported. A terminal run expires after 600 seconds; active workers never expire. The UI pages by 50 candidates and supports page selection and invalid-metadata acknowledgement. Identical managed content is hidden separately using inspected current payload bytes rather than cached metadata digests; distinct content sharing a slug remains visible. Unreadable or blocked managed payloads cannot hide a discovered source. A failed worker launch becomes a retained failed run and permits retry. Detailed scan exclusions and failures belong in redacted logs.

## Reviewed imports

Preparation re-inspects opaque selected candidates and creates a durable immutable plan valid for 300 seconds. Blocked payloads are refused; invalid metadata requires acknowledgement. Review shows source, destination, validation, size, and duplicate handling. Pending application prevents dismissal. The result survives navigation and restart until dismissed.

Application rechecks source identity, source paths, manifests, library revision, and the actual managed catalog. Every source is staged before the first managed move. Internal links become regular entries. Sources remain byte-identical. Identical content attaches an observation; distinct content sharing a slug creates a separate managed copy. Applying a completed plan returns the stored result, including after restart or plan expiry.

Payload policy preserves the captured limits, portable names, Unicode case collisions, YAML restrictions, plugin exclusions, executable flags, and explicit directories. The canonical digest is `sha256:` followed by the schema-1 manifest hash.

## Library and organization

Library supports text search, flat folder pages, Unfiled, multi-selection, assignment, folder creation and rename, and revision-checked deletion previews. Deleting a folder unfiles its skills. Names are trimmed and unique under NFKC plus full Unicode case folding. Portable tags remain stored and validated without tag-management UI.

Skill details show source observations, readers, validation, counts, and bounded file previews. Listing is limited to 5,000 entries and depth 32; text is limited to 128 KiB. Binary and oversized files remain listed with an explanation. Relative-path and canonical containment checks reject traversal and escaping links.

Shared-slug resolution previews the copies, chooses one winner, validates the expected identities and revision, and moves losers to app-local backups outside the Git repository. Source observations are never used to overwrite original skill directories.

## Synchronization

Connect, Refresh, Pull, Push, Sync, and Disconnect use a verified absolute Git executable with a 30-second process timeout. Status refreshes every 30 seconds and on window focus. User credential helpers remain available; prompts are disabled and credentials are not stored in application settings. App-owned Git attributes disable filters, line-ending conversion, ident expansion, and working-tree encoding so portable payload bytes survive synchronization. Remote URLs and branches are validated. The selected remote is reconciled with `origin`.

Refresh fetches without committing or merging. Pull is fast-forward only. Divergence and dirty managed files behind the remote require manual reconciliation. A newly initialized, empty, unborn library can adopt a populated remote through explicit Pull or Sync; a durable adoption journal protects its initial metadata.

Imports and organization changes remain uncommitted until explicit Push or Sync. Commits cover only `.skillbinder.json` and `skills/`, including managed files matched by ignore rules. Unrelated staged files block synchronization without changing the index. Sparse checkout prevents unrelated remote paths from materializing. Existing unrelated local files are preserved.

## Durability and diagnostics

One worker owns bundled SQLite and serializes library and Git mutations. Discovery remains independent. Journals persist applying or rolling-back direction, next-move intent, and expected physical identity and digest. Recovery completes or reverses interrupted moves before allowing further mutations. Unknown states retain their journal and block writes. Recovery refuses externally changed portable metadata before moving payloads or replacing metadata. A committed database transaction is never rolled back through filesystem-only recovery.

Portable schema 2 stores folders, tags, and skills as maps keyed by identity. Machine-local metadata, plans, outcomes, observations, roots, preferences, logs, staging, journals, and backups live in platform-local storage. No old database is migrated.

Logs use a bounded queue, 5 MiB rotation, at most five files, and 14-day retention. Home paths and common credential formats are redacted. Settings reveals only the engine-owned log directory. Safe errors include category, recovery action, retryability, and diagnostic identity.

## Acceptance evidence

Engine unit tests cover manifest and payload policy, local bare-remote Git behavior, redaction, and injected crash or I/O failures during forward and rollback recovery. Public-operation integration tests exercise setup, roots, shared reader discovery, cancellation, stale imports, byte preservation, replay after restart, conflicts, Unicode folder names, assignment, stale deletion, bounded previews, and concurrent engine refusal.

Native interaction, each platform build, and packaged-app smoke tests are separate evidence categories. Current results and remaining platform checks are recorded in [the verification guide](../native-gate.md). Captured fixtures alone do not establish engine or native interaction behavior.

Bindings, deployment screens, nested folders, and tag-management UI were not shipped and are outside this rewrite's feature inventory.
