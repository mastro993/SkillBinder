# Library folders beside presets

Library folders are a second, separate grouping of library skills, used only to browse. Presets are meant to become what a workspace or agent applies, so making them also the browsing grouping would give one concept two jobs. A folder never applies, and a skill is in at most one folder, as in a filesystem. A skill can still be in any number of presets. Folders are portable organization data, like presets. They live in schema 3, not a new schema, because no release has written schema 3 yet (ADR 0002). That boundary has to move if a release ships before folders do.

The new keys are a top-level `libraryFolders` map and an optional `libraryFolderId` on each skill. Schema 3 still reads schema 2's `folders` and `folderId` as aliases for `presets` and `presetIds`, so the plain names would load folder data as presets.

## Considered Options

- Moving presets into the Library header and calling them folders was rejected. It would merge browsing and applying into one concept just after presets were separated out to be applied.
- Reviving the dormant portable tags as folders was rejected. Tags are many-to-many, and tag-management UI is outside the approved scope.
- Keeping folders on each device, in preferences, was rejected. It is library organization, and a second device would lose it.
