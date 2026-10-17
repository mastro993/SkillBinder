# Only skill directories sync

The library stops having a metadata file. `.skillbinder.json` is removed, and only the skill directories under `skills/<slug>/` sync through Git. Everything else about the library is stored in the machine-local database: presets and their memberships, order, color, and icon; library folders and filing; cached digests and sizes; and install times. A skill's slug, which is its directory name, identifies it on every device. When the engine opens, and after every Pull or Sync, it reconciles the database with `skills/`. A directory that arrives becomes a library skill, unfiled and in no preset. A directory that disappears takes its local organization with it. Only skill directories that Git reports as changed are rehashed.

This supersedes ADR 0002 and ADR 0003. Schema 3, the schema 2 upgrade, and portable presets and folders are gone. No release has shipped and no library is migrated: an existing data directory or library is discarded.

Because a directory name can't repeat, one library can no longer hold two skills with the same slug. Shared-slug resolution is removed. If two devices install the same slug with different content, the next Pull reports the divergence, like any other change to managed files.

The machine-local database moves from rusqlite to Diesel's synchronous SQLite backend, still owned by the single engine worker, with embedded migrations. Operation journals move from JSON files into the database, so a journal's commit marker and the metadata change it describes share one transaction. Payload moves are still journaled and recovered.

## Considered Options

- Keeping a portable metadata file next to the database was rejected. Two stores of the same organization data need ordering rules on every write and on recovery, and a merge on Pull is needed whenever two devices edit organization data.
- Syncing presets and folders through a separate synced database file was rejected. SQLite files can't be merged by Git.
- Keeping a UUID skill identity mapped to slugs was rejected. Names already can't conflict inside a library, so the indirection would only support shared-slug resolution, which is gone.
- Rehashing every skill when the engine opens was rejected for its cost on large libraries. SkillBinder owns `skills/`; changes made to it outside the app are unsupported.
