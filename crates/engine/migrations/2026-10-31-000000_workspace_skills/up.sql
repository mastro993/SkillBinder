-- Each workspace's agents: the workspace skill directories it uses, relative to its folder, such as
-- `.claude/skills`. Workspaces added before this table existed have none until they are added again.
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
