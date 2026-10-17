CREATE TABLE device_settings(key TEXT PRIMARY KEY,value TEXT NOT NULL);
CREATE TABLE operation_plans(id TEXT PRIMARY KEY,payload TEXT NOT NULL,expires_at INTEGER NOT NULL,consumed INTEGER NOT NULL DEFAULT 0);
CREATE TABLE idempotency_records(operation_id TEXT PRIMARY KEY,request_hash TEXT NOT NULL,result TEXT NOT NULL);
CREATE TABLE skill_metadata(skill_id TEXT PRIMARY KEY,description TEXT NOT NULL,validation TEXT NOT NULL,updated_at INTEGER NOT NULL);
CREATE TABLE workspaces(id TEXT PRIMARY KEY,canonical_path TEXT NOT NULL UNIQUE,name TEXT NOT NULL,position INTEGER NOT NULL,created_at INTEGER NOT NULL);
