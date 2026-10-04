# Native architecture

The executable embeds one engine in its process. There is no daemon or network command transport. The native workspace implements every audited shipped journey: onboarding, Discovery and imports, Library and flat folders, Sync, and Settings.

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

The engine keeps storage and system adapters private within feature modules. A single worker owns the bundled SQLite connection. Its typed job queue serializes changes to managed files, metadata, and the Git working tree. Discovery runs independently. Bounded previews run on the engine worker and never block the GPUI event loop.

The application stores machine-local state beneath the operating system's local application-data directory in `SkillBinder`. The portable library uses `.skillbinder.json` schema 2 and `skills/<slug>/`. There is no old-database migration or compatibility layer.

Navigation never cancels committed work. The engine retains terminal import outcomes, journals durable mutations, and replays completed imports idempotently. Recovery must reconcile interrupted operations before enabling further mutations. Scan retention begins after terminal completion, cancellation, or failure.

The client publishes immutable projections through coalescing subscriptions. Full refresh generations reject superseded reads; scan updates are read while holding the publication lock so older observations cannot replace newer scans. Failed projection refreshes never turn a completed mutation into a failure. Detailed acceptance cases remain in `fixtures/contracts/domain-contract.md`.

The executable drains the engine in GPUI’s quit hook before native termination. This is necessary on macOS, where AppKit can terminate the process without returning from the application event loop. Quit shortcuts and last-window closure share this path. The worker closes SQLite and explicitly releases the library lock before shutdown returns. This also releases inherited references held by subprocesses; closing only the worker's descriptor would leave those references locked.

## Dependency boundary

Ely remains unmodified at `2f8b2f687cd1e9b98a7cd29d4e882d09406fc547`. Both Cargo GPUI patch tables select the GPUI copy in `gpui-mcp` revision `9dda8e5cb49990261e3fdaa26abe38112d30dafb`. The `gpui_platform` dependency remains at official revision `1a28cff4b409169bac058bca40dfbfeb7621d19b`. The patch affects ordinary builds too, although the optional bridge dependency is present only with `native-test`.

The application enables `font-kit`, X11, and Wayland through supported platform features. Ely remains unmodified. The application asset source maps each exercised icon path to licensed Hugeicons artwork and rejects an unmapped icon path.

Ely registers its bundled fonts during initialization. The application then sets the native system font explicitly so the component default does not change the production baseline's typography.
