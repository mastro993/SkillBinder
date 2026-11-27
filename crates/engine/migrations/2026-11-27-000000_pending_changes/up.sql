-- Why each skill changed on this device since the last commit Push or Sync made: one row per
-- slug, written by the import, install, update, reinstall, or record write that changed it, and
-- cleared by the commit that publishes it. `reason` is the word a commit trailer carries, and
-- `detail` what follows it, such as the agent or workspace an import found the skill in.
CREATE TABLE pending_changes (
    slug TEXT PRIMARY KEY NOT NULL,
    reason TEXT NOT NULL,
    detail TEXT,
    at TEXT NOT NULL
);
