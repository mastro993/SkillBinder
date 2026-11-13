---
name: sql-review
description: Reviews SQL migrations for PostgreSQL — locks, indexes, and data backfills that should run outside the migration.
---

# SQL review (PostgreSQL)

- Prefer `CREATE INDEX CONCURRENTLY` on tables over a million rows.
- Move data backfills into a separate, resumable job.
- Check that every `ALTER TABLE` sets a `lock_timeout`.
