-- Who a managed copy record is held for: the workspace, and the workspace skill directory it is
-- held through, such as `.claude/skills`, or NULL for your agents. Removing a workspace forgets the
-- records held for it alone, leaving their copies; one another place still holds passes to it
-- first. A record whose directory its holder's directory no longer leads to, as after a link in
-- the project changed, is forgotten without touching it.
ALTER TABLE managed_copies ADD COLUMN workspace_id TEXT REFERENCES workspaces(id) ON DELETE CASCADE;
ALTER TABLE managed_copies ADD COLUMN workspace_dir TEXT;
