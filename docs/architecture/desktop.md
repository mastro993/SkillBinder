# Native desktop architecture

The Cargo workspace has one application, `apps/desktop`. Every UI definition lives in `crates/ui`.
The desktop host creates the service graph, installs redacted logging, initializes GPUI, opens one
window, and handles activation/quit. `crates/ui` owns shared components, feature screens, native
folder selection, semantic colors, icons, dialogs, input, and presentation state.

Typed application actions and models live in `crates/app`. The UI schedules blocking actions on
GPUI's background executor and applies results through weak entities on the UI thread. A window
that has closed cannot receive a late update. Operation keys suppress duplicate submissions.
Discovery workers and sessions are owned by the app state, so navigation does not cancel a scan.

| Existing flow | Native implementation |
| --- | --- |
| Resumable setup and Git/storage repair | `screens/onboarding.rs`; core transition rules retained |
| Recovery blocking and diagnostic access | Onboarding recovery view and Settings |
| Single instance and focus | platform process lock and activation marker; desktop lifecycle |
| System light/dark appearance | `theme.rs`, OS appearance observer, original semantic palette |
| Discovery, progress, cancellation and limits | `screens/discovery.rs`, app worker and 400 ms polling |
| Pagination and selection across pages | presentation `Selection`, 100-candidate pages |
| Invalid opt-in and blocked candidates | UI selection rules plus authoritative core validation |
| Review/apply, expiry and source identity checks | native modal; persisted single-use plans and unchanged import service |
| Library validation, source readers and dirty state | `screens/library.rs` |
| Conflict choice, timestamps, file previews and backups | `components/review.rs`, library maintenance |
| Folder pick/register, rename, enable and remove | `screens/settings.rs`; native picker, single-use grant |
| Connect/refresh/pull/push/sync/disconnect | `screens/git_sync.rs`; existing local Git authentication |
| Redacted rotating logs and folder reveal | app logging, platform log sink and reveal adapter |

Cancel, backdrop, and Escape dismiss a review only when no apply operation is running. The modal
restores keyboard focus on close. Text input uses the native component editor with selection,
clipboard, and IME support. Preview payloads are plain inert text; copying content is explicit.

Tests retain the domain, storage, platform, and application behavior coverage. Presentation tests
exercise selection and confirmation rules. The three journey probes use isolated real files,
SQLite, and Git. Architecture verification checks allowed workspace roots and dependency direction.
