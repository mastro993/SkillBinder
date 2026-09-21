# 007 - Provider registry and shared paths

Status: accepted, 2026-09-20.

## Decision

Discovery reads a declarative registry checked into the repository at
`crates/core/src/discovery/registry.json`, pinned to `vercel-labs/skills` commit
`7407f3893ad4dceab546ac002c3ef806e4000c73`. Every agent ID in that upstream `AgentType` union has
one entry. No part of the upstream JavaScript runtime is bundled or executed.

Each entry names the agent, its project skills directory, and its global skill roots as path
templates. Core resolves the templates from the OS home directory and the process environment
without a shell. `~` is the home directory, `${VAR}` and `${VAR:-default}` read environment
variables once at scan time.

Locations are deduplicated by resolved physical path, not by agent. One physical directory appears
once and lists every agent that reads it. `.agents/skills` is the common case; several agents share
it.

`node scripts/registry.mjs check` re-derives the fact table from the pinned upstream snapshot in
`tests/fixtures/upstream-skills-cli/<commit>/` and fails when an ID, display name, project
directory, or global root differs from the registry. CI runs it. The same script regenerates
`docs/agent-support.md`.

Where upstream chooses one existing directory out of several candidates, the app lists every
candidate in upstream precedence order and scans each one that exists. OpenClaw is the current
case with `.openclaw`, `.clawdbot`, and `.moltbot`. A legacy directory that still holds skills
stays visible to the user instead of disappearing.

## Alternatives

Declarative data was chosen over a plugin interface with executable adapters. Executable adapters
would let a registry update change app behavior at runtime and would put untrusted code in the
scan path for no benefit, since path rules are data.

Per-agent scanning was rejected in favor of physical deduplication, because the same directory read
by four agents is one location with four reader labels, not four discoveries.

Trusting the upstream package at runtime was rejected. The app ships no Node.js requirement and
must not execute upstream detection functions.

## Consequences

Registry updates are reviewed source changes followed by `node scripts/registry.mjs docs` and a
green `check`. A registry change invalidates pending plans, which revalidate at apply time.

The registry records three separate statuses per agent: documented, path-tested, and
runtime-tested. This revision claims documented only. Writing files for an agent never proves that
the agent loads them.

## Reversal cost

Replacing the registry with a different source means re-deriving the JSON and the comparison half
of `scripts/registry.mjs`. Consumers only depend on the resolved root list, so the scan, import,
and frontend layers do not change.
