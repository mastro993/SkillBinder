-- Whether a library skill is New: installed on this device and not yet activated anywhere or
-- opened. Machine-local; it never syncs. Skills already in the library are not New.
ALTER TABLE skills ADD COLUMN new BOOLEAN NOT NULL DEFAULT 0;
