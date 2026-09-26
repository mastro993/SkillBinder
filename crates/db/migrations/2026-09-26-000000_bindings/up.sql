CREATE TABLE binding_actions (
  id TEXT PRIMARY KEY,
  skill_ids TEXT NOT NULL,
  scope TEXT NOT NULL,
  project_root_id TEXT,
  agent_ids TEXT NOT NULL,
  target_paths TEXT NOT NULL,
  created_at INTEGER NOT NULL
);
CREATE TABLE binding_receipts (
  target_path TEXT PRIMARY KEY,
  skill_id TEXT NOT NULL,
  digest TEXT NOT NULL
);
