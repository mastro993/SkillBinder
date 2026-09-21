# Phase 3, line redactor

Back to [overview](overview.md). Depends on phase 1.

## Goal

Strip what the spec forbids from every line before it reaches disk, in one place that every record passes through.

## Changes

`crates/platform/src/redact.rs` is new, wired by `crates/platform/src/lib.rs`. The worker from phase 2 calls it on each complete line, so the rewrite happens exactly once per line and no caller can skip it.

Rules, each one mechanical and testable.

1. Replace the home directory prefix with `~`, in both the plain form and the JSON-escaped form, so Windows separators are covered too.
2. Replace URL userinfo with `***`, turning `https://user:token@host/path` into `https://***@host/path`.
3. Replace the values of credential-shaped query parameters such as `token`, `access_token`, `secret`, `password`, and `key` with `***`.
4. Replace bearer tokens, both the `Bearer <value>` form and a bare `Authorization` header value, with `***`.

The rewrite is idempotent, so a line that matches twice is unchanged the second time.

What the redactor does not do, stated so nobody mistakes it for a guarantee. It cannot detect a file body, a draft, or a private repository name, because those are ordinary strings. Those stay a caller rule, and the seam in phase 6 keeps the most common source of them, `AppError.message`, out of the log entirely.

## Data structures

```rust
pub struct Redactor { home: String, home_json: String }
impl Redactor {
    pub fn new(home: &Path) -> Self;
    pub fn rewrite_line(&self, line: &str) -> String;
}
```

## Verification

Static. `cargo test -p skillbinder-platform` and clippy as in phase 1.

Runtime, in unit tests. Each rule gets a case that would fail if the rule were dropped, plus a clean line that must come back unchanged, plus a case where the home prefix appears inside a JSON string. A test that runs the same line through twice asserts idempotence.
