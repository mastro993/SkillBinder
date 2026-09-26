# Bindings

A binding is a machine-local action that copies selected canonical library skills into agent skills folders. A single action may include several skills and agents. The Bindings page lists every action separately, including repeated actions for the same skill and folder. It never creates links.

For global bindings, the app resolves target folders from the bundled agent registry and local environment. For project bindings, the user selects an already registered project root or chooses a directory through the native folder picker. The frontend sends root and agent IDs, never a destination path. The backend resolves each destination from the registered root and registry rule.

Several agents may read one physical folder. The app writes one copy per physical skill path and lists all known readers. A receipt in local SQLite records the skill ID and manifest digest for each managed copy; binding actions live in a separate table so they can overlap. Neither table enters the portable library. A target already occupied by an unmanaged, changed, or symlinked entry is left untouched.

The Bindings page refreshes status while open and on focus. A deleted managed copy appears as **missing** and can be restored with **Repair**. Repair rechecks the saved target against the current registered project or global registry path and the canonical source manifest before writing. Changed copies require manual resolution. Repair never overwrites them.
