# Native architecture

The executable embeds one engine in its process. There is no daemon or network command transport. The dependency gate currently implements the executable, UI integration, theme setup, and repository tooling.

## Target ownership

| Package | Responsibility |
| --- | --- |
| `apps/skillbinder` | Native lifecycle, configuration, Tokio runtime, engine lifetime, process lock, and orderly shutdown |
| `crates/proto` | Shared IDs, typed inputs and results, immutable snapshots, and safe structured errors |
| `crates/engine` | Feature-oriented domain operations, jobs, persistence, filesystem operations, Git, and recovery |
| `crates/client` | Typed operations, cached projections, coalesced subscriptions, request generations, and stale-read rejection |
| `crates/ui` | GPUI entities for navigation, filters, selection, focus, dialogs, and native integration |
| `crates/theme` | Colors, typography, dimensions, and motion |
| `xtask` | Verification, registry documentation, fixtures, and packaging |

The proto, engine, and client packages do not exist yet. Creating them during the dependency gate would add unused interfaces before their behavior can be verified.

## Execution and storage contract

The engine keeps storage and system adapters private within feature modules. A single worker owns the bundled SQLite connection. A mutation coordinator serializes changes to managed files, metadata, and the Git working tree. Discovery and bounded reads remain independent.

The application stores machine-local state beneath the operating system's local application-data directory in `SkillBinder`. The portable library uses `.skillbinder.json` schema 2 and `skills/<slug>/`. There is no old-database migration or compatibility layer.

Navigation never cancels committed work. The engine retains terminal import outcomes, journals durable mutations, and replays completed imports idempotently. Recovery must reconcile interrupted operations before enabling further mutations. Scan retention begins after terminal completion, cancellation, or failure.

These paragraphs define the accepted target, not implemented engine behavior. Detailed acceptance cases remain in `fixtures/contracts/domain-contract.md`.

## Dependency boundary

Ely is pinned to `2f8b2f687cd1e9b98a7cd29d4e882d09406fc547`. GPUI and `gpui_platform` use official revision `1a28cff4b409169bac058bca40dfbfeb7621d19b`. Cargo resolves one graph and one lockfile.

The application enables `font-kit`, X11, and Wayland through supported platform features. Ely remains unmodified. The application asset source maps each exercised icon path to licensed Hugeicons artwork and rejects an unmapped icon path.

Ely registers its bundled fonts during initialization. The application then sets the native system font explicitly so the component default does not change the production baseline's typography.
