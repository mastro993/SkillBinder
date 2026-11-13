---
name: sql-review
description: Reviews SQL migrations for locking, missing indexes, and irreversible steps before they reach production.
---

# SQL review

- Flag statements that take an exclusive table lock on large tables.
- Ask for an index on every new foreign key.
- Require a down migration, or a note saying why there is none.
