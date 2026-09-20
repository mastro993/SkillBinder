# 014 - Complete agent coverage

Status: accepted, 2026-09-20.

## Decision

The MVP supports every agent in the pinned upstream registry, currently 79 IDs, with no curated
subset. `node scripts/registry.mjs check` compares the exact upstream ID set with the implemented
set and fails when an ID is missing. `docs/agent-support.md` is generated from the same data, so
the documentation cannot drift from the registry.

An agent whose upstream entry has no global directory, because the agent is project-scoped, gets an
empty global root list. The app never invents a global directory for a project-only agent.

Shared directories are scanned once and displayed once with all known readers. Coverage is a claim
about the recorded snapshot, not about unknown future agents.

## Alternatives

Shipping a six-agent subset was the earlier proposal and was rejected by the owner. It passes
quickly and fails acceptance, and it hides real shared-path behavior, which is exactly the part
that needs testing.

Bulk-installing into every agent was rejected. Coverage of the registry is about inspection and
target resolution, not about deploying to every agent by default.

## Consequences

Every registry entry needs a resolution fixture and, when an agent stops supporting a global scope,
an explicit empty list rather than a silent delete. Adding a new agent means editing
`registry.json`, re-running the coverage check, and regenerating `docs/agent-support.md`.

## Reversal cost

Narrowing coverage means deleting entries and relaxing the check. Nothing else depends on the size
of the set.
