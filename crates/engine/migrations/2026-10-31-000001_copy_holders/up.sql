-- The workspace a managed copy record is held for, or NULL for your agents. Removing a workspace
-- forgets the records held for it alone, leaving their copies; one another place still holds
-- passes to it first.
ALTER TABLE managed_copies ADD COLUMN workspace_id TEXT REFERENCES workspaces(id) ON DELETE CASCADE;
