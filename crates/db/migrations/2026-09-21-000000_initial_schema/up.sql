-- The machine-local state schema, applied on connect.
--
-- Every table is created with IF NOT EXISTS: a database written by the pre-Diesel build already has
-- them but no Diesel bookkeeping, so this migration is what upgrades such an install in place.
CREATE TABLE IF NOT EXISTS device_settings(key TEXT PRIMARY KEY,value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS operation_plans(id TEXT PRIMARY KEY, payload TEXT NOT NULL, expires_at INTEGER NOT NULL, consumed INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS idempotency_records(operation_id TEXT PRIMARY KEY, request_hash TEXT NOT NULL, result TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS source_observations(id INTEGER PRIMARY KEY, source TEXT NOT NULL, skill_id TEXT NOT NULL, digest TEXT NOT NULL, warnings TEXT NOT NULL, reader_agents TEXT NOT NULL, observed_at INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS skill_metadata(skill_id TEXT PRIMARY KEY, description TEXT, validation TEXT NOT NULL, updated_at INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS scan_roots(id TEXT PRIMARY KEY, canonical_path TEXT NOT NULL UNIQUE, display_path TEXT NOT NULL, label TEXT NOT NULL, enabled INTEGER NOT NULL DEFAULT 1, created_at INTEGER NOT NULL);
-- The pre-Diesel build created this table and never wrote to it. Diesel tracks applied migrations in
-- __diesel_schema_migrations instead.
DROP TABLE IF EXISTS schema_migrations;
