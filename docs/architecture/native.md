# Native architecture

The executable embeds one engine in its process. There is no daemon or network command transport. The native workspace implements every audited shipped journey: onboarding, Discovery and installs, Library and presets, Sync, and Settings. It also implements workspaces and the agents' global skill directories, which every scan reads, and the Agents view of found skills by detected agent.

## Ownership

| Package | Responsibility |
| --- | --- |
| `apps/desktop` | Native lifecycle, configuration, Tokio runtime, engine lifetime, process lock, and orderly shutdown |
| `crates/proto` | Shared IDs, typed inputs and results, immutable snapshots, and safe structured errors |
| `crates/engine` | Feature-oriented domain operations, jobs, persistence, filesystem operations, Git, and recovery |
| `crates/client` | Typed operations, cached projections, coalesced subscriptions, request generations, and stale-read rejection |
| `crates/ui` | GPUI entities for navigation, filters, selection, focus, dialogs, and native integration |
| `crates/theme` | Colors, typography, dimensions, and motion |
| `xtask` | Verification, registry documentation, fixtures, and packaging |

The UI invokes typed client operations. It never accesses SQLite, mutates filesystem content, or starts Git processes. Domain adapters remain private to feature-oriented engine modules.

## Execution and storage contract

The engine keeps storage and system adapters private within feature modules. A single worker owns the bundled SQLite connection. Its typed job queue serializes changes to managed files, metadata, and the Git working tree. SkillBinder never writes outside application storage and the library. One scan of every registry agent's global skill directories and every workspace runs on its own thread: the worker prepares its inputs, and the scan publishes its found skills to a shared in-memory record that the client reads without waiting for the worker. Each found location records the agents that read it, the agents whose own global skill directory, the one the app shows, holds it through any link, and the workspaces whose scan reads it; the workspace list is empty for a global skill directory. The UI derives agent pages, sidebar counts, and presence from those agent and workspace identities, never from display paths. Found skills are scan observations and are never stored. Starting a scan replaces a running one, whose results are discarded. Bounded previews run on the engine worker and never block the GPUI event loop.

The application stores machine-local state beneath the operating system's local application-data directory in `SkillBinder`. The portable library uses `.skillbinder.json` schema 3 and `skills/<slug>/`; reads upgrade schema 2 libraries, as `docs/adr/0002-portable-schema-3-for-presets.md` records. There is no old-database migration or compatibility layer.

Navigation never cancels committed work. The engine retains terminal install outcomes, journals durable mutations, and replays completed installs idempotently. Recovery must reconcile interrupted operations before enabling further mutations. The last completed scan is retained until the next one completes.

The client publishes immutable projections through coalescing subscriptions. Full refresh generations reject superseded reads; scan results are read while holding the publication lock, so older results cannot replace newer ones. On creation, and when the window regains focus, the client starts a scan in the background unless one is already running. Failed projection refreshes never turn a completed mutation into a failure. Detailed acceptance cases remain in `fixtures/contracts/domain-contract.md`.

The executable drains the engine in GPUI’s quit hook before native termination. This is necessary on macOS, where AppKit can terminate the process without returning from the application event loop. Quit shortcuts and last-window closure share this path. The worker closes SQLite and explicitly releases the library lock before shutdown returns. This also releases inherited references held by subprocesses; closing only the worker's descriptor would leave those references locked.

## Dependency boundary

The application depends on GPUI Kit 0.7.0. Kit re-exports the matching GPUI Pre 0.3.7 types, platform bootstrap, Base infrastructure, and styled components. Application crates import these APIs through `gpui_kit`; there is no separate component or platform dependency.

The Cargo `gpui-pre` patch selects the backend supplied by `gpui-mcp` revision `9dda8e5cb49990261e3fdaa26abe38112d30dafb`. The optional bridge selects its supported `gpui-pre` backend with default features disabled. Kit, its platform, and the bridge therefore share one GPUI type graph. The backend patch also applies to ordinary builds, while the bridge itself is present only with `native-test`. No dependency source is modified.

Kit enables `font-kit`, X11, Wayland, and runtime shaders through its platform dependency. The application asset source maps exercised component icon paths to licensed Hugeicons artwork and rejects unmapped icons. It does not fall back to a different icon catalog or bundle component fonts. The application sets the native system font explicitly and follows window appearance through the public theme API.

Kit's `open_window` installs its `Root` around the application view. Root owns keyboard traversal and the native dialog layer. The view owns `InputState` entities and subscriptions across renders; input changes use Kit's window-aware editing API. Typed dialog state remains in the view, and a single window dialog builder reads its current state on each render. Dismissal restores prior focus. Review dialogs use explicit footer actions and refuse implicit confirmation from Enter on the modal container. Escape, backdrop dismissal, close controls, and footer cancellation are disabled while a dialog operation is pending.
