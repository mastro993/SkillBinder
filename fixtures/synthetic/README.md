# Synthetic world

A small fictional machine for native-test runs, so testing never reads or writes your own agents, projects, or library. Every skill, project, and person here is made up.

Build it with `cargo run -p xtask --locked -- fixture <new directory>`, then launch with the command it prints (see [native checks](../../docs/native-gate/checks.md#launch-on-the-synthetic-world)). The task writes:

| Path | Built from | What it exercises |
|---|---|---|
| `home/` | `home/` here | Agents' global skill directories: Claude Code, Cursor, Universal (`.agents/skills`), GitHub Copilot, and Gemini CLI with no skills. |
| `home/.agents/skills/` | `home/` here and the first snapshot of each source | Skills a `skills` CLI install would leave, with `home/.agents/.skill-lock.json` naming their source. |
| `projects/` | `projects/` here | Pick this folder to add `atlas-web` and `orbit-api` as workspaces; `scratch-notes` has no agent skill directory and is skipped. |
| `sources/<name>.git` | `sources/<name>/<n>/`, one commit per numbered snapshot, oldest first | A local skill source: the installed skills come from snapshot 1, so the skills snapshot 2 changes show an update. |
| `remotes/<name>.git` | `remotes/<name>/`, one commit | A Sync remote holding a library pushed from another machine. |
| `data/` | nothing | Created by the app on first launch: start with onboarding. |

States the world holds:

- **Same slug, different content:** `sql-review` in Claude Code and in Cursor.
- **Invalid:** `broken-frontmatter` has malformed YAML.
- **Warning:** `oncall-handbook`'s description exceeds 1,024 characters.
- **Blocked:** `plugin-bundle` holds a `.claude-plugin/plugin.json`.
- **Updates:** `api-design` and `incident-runbook` change in the source's snapshot 2.
- **Workspace copy of a library skill:** `atlas-web` holds `release-notes` with the same content as Claude Code's.

Descriptions avoid `: `, which ends a plain YAML scalar, except where a skill is meant to be invalid. A `.gitkeep` keeps an empty directory in Git; the task creates the directory without it.
