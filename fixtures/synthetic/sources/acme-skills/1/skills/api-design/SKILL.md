---
name: api-design
description: Designs HTTP APIs the Acme way — resource names, pagination, errors, and versioning.
---

# API design

- Name resources with plural nouns.
- Paginate with an opaque `cursor`, never page numbers.
- Return errors as `{ "error": { "code", "message" } }`.
