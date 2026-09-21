# Skill provenance

## What a checked-in skill is

A checked-in skill is a directory under `.agents/skills/<name>/` that ships with the repository and
is read from the working tree. Nothing is installed, fetched, or executed to load one.

Every skill holds a `SKILL.md` at its root. The supporting files vary by skill and are copied along
with it:

```text
.agents/skills/
├── rust-async-patterns/          SKILL.md, references/
├── rust-best-practices/          SKILL.md, references/
├── shadcn/                       SKILL.md, cli.md, mcp.md, registry.md,
│                                 customization.md, rules/, agents/, assets/, evals/
├── tanstack-router/              SKILL.md (first-party, not vendored)
├── tauri-v2/                     SKILL.md, README.md, references/
├── vercel-react-best-practices/  SKILL.md, README.md, AGENTS.md, rules/
└── zod/                          SKILL.md, README.md, AGENTS.md, references/, assets/
```

One skill is first-party and the rest are vendored. A vendored skill is a copy of an upstream
package and carries a lock entry. A first-party skill is written for this repository and carries
none. `.claude/skills/<name>` is a relative symlink to the same directory when an agent reads that
location instead.

`.agents/` is exempt from the format and lint gates: it is listed in `ignorePatterns` of both
`.oxfmtrc.json` and `.oxlintrc.json`. Vendored content is reviewed, not reformatted.

## skills-lock.json

`skills-lock.json` at the repository root is version 1 and holds a `skills` map keyed by skill
name. Each entry records `source` (the GitHub `owner/repository`), `sourceType` (`github`),
`skillPath` (the `SKILL.md` inside that repository), and `computedHash`.

The six entries cover the six vendored skills above, so the lock file and `.agents/skills/` are
read together. The lock records vendored provenance only: a first-party skill such as
`tanstack-router` has no entry. The hash is the provenance record of the upstream revision that was
reviewed. It is not a digest of the checked-in copy: the local `SKILL.md` files hash differently,
and no script, package script, or test reads `skills-lock.json` today. Replace the entry when a
vendored skill is replaced, and keep the path and the hash with it.

## Trust rule

Load reviewed skills from the working tree only. A vendored skill enters `.agents/skills/` through a
reviewed commit that also updates `skills-lock.json`. A first-party skill enters through a reviewed
commit alone.

Never run an unpinned remote skill loader. `pnpm dlx @tanstack/intent@latest` is the named example
in `apps/frontend/AGENTS.md`. If remote package execution is genuinely required, use an explicitly
reviewed pinned version and get operator approval before running it.

When a task matches a checked-in skill, use it before falling back to general knowledge.
`apps/frontend/AGENTS.md` is the routing example for frontend work.

## TanStack Router

Routing work in `apps/frontend` starts with `.agents/skills/tanstack-router/SKILL.md`, which
`apps/frontend/AGENTS.md` names for substantial routing and frontend-framework tasks. It is
first-party and covers file-based routes under `src/routes`, the router instance, and
`src/routeTree.gen.ts`, pinned to the TanStack Router versions installed in `apps/frontend`.

`.claude/skills/tanstack-router` is a relative symlink to that directory, so both agent layouts read
the same file. The skill has no `skills-lock.json` entry, because the lock records vendored
upstream provenance and this skill is written in this repository.
