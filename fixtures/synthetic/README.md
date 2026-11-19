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
| `repositories/<name>.git` | `repositories/<name>/`, one commit | A repository to add under Explore's Your repositories: `team-skills` lists 14 skills. |
| `catalog/` | `catalog/` here | Explore's catalog, laid out like the CDN's `v1/`: four fictional creators and their skills, three with avatars. Launch with `--catalog-dir`; rename the folder to go offline, or delete `catalog/v1` to need a newer app. |
| `data/` | nothing | Created by the app on first launch: start with onboarding. |

States the world holds:

- **Same slug, different content:** `sql-review` in Claude Code and in Cursor.
- **Invalid:** `broken-frontmatter` has malformed YAML.
- **Warning:** `oncall-handbook`'s description exceeds 1,024 characters.
- **Blocked:** `plugin-bundle` holds a `.claude-plugin/plugin.json`, and `team-skills`' `legacy-plugin` a `.codex-plugin/plugin.json`, so Install all ends with one failure.
- **Updates:** `api-design` and `incident-runbook` change in the source's snapshot 2.
- **In library in Explore:** `pdf-tools` in the universal agents' directory records Northwind Labs' repository, which the catalog lists it from; once imported, its Explore card shows "In library". Its host never resolves, so its update check fails.
- **Workspace copy of a library skill:** `atlas-web` holds `release-notes` with the same content as Claude Code's.

Descriptions avoid `: `, which ends a plain YAML scalar, except where a skill is meant to be invalid. A `.gitkeep` keeps an empty directory in Git; the task creates the directory without it.
