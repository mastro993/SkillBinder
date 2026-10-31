-- Each workspace's agents: the workspace skill directories it uses, relative to its folder, such as
-- `.claude/skills`. Workspaces added before this table existed have none: their first activation
-- chooses them, and adding them again adopts the copies they hold.
CREATE TABLE workspace_dirs(
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    dir TEXT NOT NULL,
    PRIMARY KEY(workspace_id, dir)
);

-- The library skills active in each workspace: for all its agents when `dirs` is NULL, or, when it
-- holds a JSON array of directories, for exactly those.
CREATE TABLE workspace_skills(
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    slug TEXT NOT NULL REFERENCES skills(slug) ON DELETE CASCADE,
    dirs TEXT,
    PRIMARY KEY(workspace_id, slug)
);

-- The workspace a managed copy record is held for, or NULL for your agents. Removing a workspace
-- forgets the records held for it alone, leaving their copies; one another place still holds
-- passes to it first.
ALTER TABLE managed_copies ADD COLUMN workspace_id TEXT REFERENCES workspaces(id) ON DELETE CASCADE;
