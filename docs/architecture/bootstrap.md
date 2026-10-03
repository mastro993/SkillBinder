# Bootstrap and onboarding

`crates/ui` renders prerequisite status and saves the selected step through `crates/app::actions`.
`apps/desktop` owns process and window lifecycle. `crates/app` wires the long-lived services;
core owns step transitions and bootstrap ports, platform owns Git/files/locks, and db owns SQLite.

1. `system_bootstrap` checks storage and Git, loads saved progress, and inspects library state.
2. `onboarding_progress_update` persists one closed-enum step.
3. `onboarding_complete_local` revalidates prerequisites, builds the initial repository in staging,
   commits portable metadata, atomically renames it, then marks onboarding complete.
4. Repeating completion returns the existing library identity and revision.

Failure before rename leaves onboarding incomplete. Retry removes only the fixed initialization
staging directory. Prerequisite failure returns the UI to the prerequisite step even when a later
step was saved. Recovery state blocks writes and offers a recheck and local log-folder access.

The process lock prevents two state owners. A second process writes a local activation marker;
the first consumes it and focuses the window. No network listener or arbitrary message handler is used.
The host obtains the OS home directory and retains the existing `~/.skillbinder` data location.
