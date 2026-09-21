# Bootstrap and onboarding

## Ownership

The frontend renders prerequisite status and persists only a step through typed commands. `apps/tauri` maps transport DTOs and composes the long-lived services. `crates/core` owns the onboarding use case, step transitions, and ports. `crates/platform` implements application storage, trusted system-Git, process lock, and local-library ports. `crates/db` implements the non-secret onboarding-state port with SQLite.

## State flow

1. `system_bootstrap` checks storage and Git, loads saved progress, and inspects library state.
2. `onboarding_progress_update` persists one closed-enum step.
3. `onboarding_complete_local` revalidates prerequisites, builds the initial repository in app staging, commits portable metadata, atomically renames it into place, then marks onboarding complete.
4. Repeating completion returns the existing library identity and revision.

Failure before the library rename leaves onboarding incomplete. A later retry removes only the fixed app-owned initialization staging directory. The process lock and Tauri single-instance plugin prevent two persistent-state owners.

## Commands

The onboarding commands are `system_bootstrap`, `git_environment_verify`,
`onboarding_progress_update`, and `onboarding_complete_local`. The main-window capability grants
these four together with the discovery, root, import, and library commands; `node
scripts/check-architecture.mjs` fails when the set drifts. No remote origin is allowed.
