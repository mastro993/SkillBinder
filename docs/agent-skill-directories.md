# Agent-specific skill directories

Findings for issue #85, retrieved 2026-11-04. They change nothing: the registry, `fixtures/contracts/registry-79.json`, still lists one workspace skill directory per agent, and [the proposal](#proposal) describes how it could record more.

## Question

[ADR 0006](adr/0006-activation-writes-managed-copies.md) makes a skill directory, not an agent, the target of activation, so a skill can be activated for one agent alone only where that agent has a directory of its own. The registry gives 24 agents the shared workspace directory `.agents/skills`, and 13 a shared global directory, `~/.agents/skills` or `${XDG_CONFIG_HOME:-~/.config}/agents/skills`. For each of them: does the agent also read a directory of its own, and what does it do when that directory and a shared one hold the same slug?

Universal is left out, because it stands for the shared directories rather than an agent, and so is PromptScript, which has no global directory. That leaves 22 agents.

## Method

- Evidence is the agent's source code, its official documentation, or its official changelog. Code and documentation kept in a repository are pinned to the full commit SHA of the latest release, or of the default branch where there are no releases. Documentation published only on a website, or published there in a newer version than its repository copy, as Droid's is, is cited as retrieved on 2026-11-04, with an archived copy where one exists. Closed-source binaries, third-party pages, and the upstream registry are not evidence.
- A claim without such a source reads "no evidence found", and same-slug behavior without one reads "unknown".
- Every quotation was checked by script against its source at the cited revision and lines. Each agent was researched once and then reviewed separately, with its sources fetched again and its code searched for directories the research missed.
- Upstream, `vercel-labs/skills` still models one directory per agent at `9197d919034d19010ae303cf96bf19ad087dd4be` (2026-11-03), so it settles none of the question.

The evidence is in [A to C](agent-skill-directories/a-c.md), [D to G](agent-skill-directories/d-g.md), and [K to Z](agent-skill-directories/k-z.md). There, "shared" means the three shared directories above; "own" means a directory named for the agent; and "other" means any other directory it reads, usually another agent's, such as `.claude/skills`. "Same slug" is one of four verdicts: **precedence**, one copy wins in an order a source gives; **one**, one copy wins but no source fixes which; **both**, the agent keeps every copy; and **unknown**.

## Summary

The last column says what the agent does when its own directory and a shared one in the same scope hold one slug.

| Agent | Own workspace directory | Own global directory | Shared global directories it reads | Same slug |
| --- | --- | --- | --- | --- |
| Amp | none | `~/.config/amp/skills` | `~/.config/agents/skills`, `~/.agents/skills` | shared wins |
| Antigravity | none | `~/.gemini/config/skills` | none found | nothing to compare |
| Antigravity CLI | none | `~/.gemini/antigravity-cli/skills`, `~/.gemini/config/skills` | none found | nothing to compare |
| Cline | `.cline/skills` | `~/.cline/skills` | `~/.agents/skills` | shared wins |
| Codex | `.codex/skills` | `~/.codex/skills`, deprecated; `/etc/codex/skills` | `~/.agents/skills` | both kept |
| Cursor | `.cursor/skills` | `~/.cursor/skills` | `~/.agents/skills` | both listed in the IDE |
| Deep Agents | `.deepagents/skills` | `~/.deepagents/agent/skills` | `~/.agents/skills` | shared wins |
| Dexto | `.dexto/skills`, legacy | `~/.dexto/skills`, legacy | `~/.agents/skills` | shared wins |
| Droid | `.factory/skills` | `~/.factory/skills` | `~/.agents/skills` | one wins, undocumented which |
| Firebender | `.firebender/skills` | `~/.firebender/skills` | `~/.agents/skills` | one wins, undocumented which |
| fx | `.fx/skills` | `~/.fx/skills` | `~/.agents/skills` | both kept |
| Gemini CLI | `.gemini/skills` | `~/.gemini/skills` | `~/.agents/skills` | shared wins, with a warning |
| GitHub Copilot | `.github/skills` | `~/.copilot/skills` | `~/.agents/skills` | shared wins in VS Code, own wins in the CLI |
| Kilo Code | `.kilo/skills`; legacy `.kilocode/skills` | `~/.kilo/skills`, `${XDG_CONFIG_HOME:-~/.config}/kilo/skills`; legacy `~/.kilocode/skills` | `~/.agents/skills` | own wins |
| Kimi Code CLI | `.kimi-code/skills` | `~/.kimi-code/skills` | `~/.agents/skills` | own wins |
| Loaf | none | `~/.loaf/skills` | `~/.agents/skills` | both kept |
| OpenCode | `.opencode/skills` | `${XDG_CONFIG_HOME:-~/.config}/opencode/skills`, `~/.opencode/skills` | `~/.agents/skills` | one wins, nondeterministic |
| Pi | `.pi/skills` | `~/.pi/agent/skills` | `~/.agents/skills` | own wins |
| Replit | `.local/secondary_skills`, picker only | none documented | none documented | unknown |
| Sarvam Code | no evidence found | no evidence found | no evidence found | unknown |
| Warp | `.warp/skills` | `~/.warp/skills` | `~/.agents/skills` | both kept, unless identical |
| Zed | none | none | `~/.agents/skills` | nothing to compare |

## What it means for SkillBinder

### Activating for one agent

In a workspace, 15 of the 22 agents read a directory of their own: Cline, Codex, Cursor, Deep Agents, Dexto, Droid, Firebender, fx, Gemini CLI, GitHub Copilot, Kilo Code, Kimi Code CLI, OpenCode, Pi, and Warp. Amp, Antigravity, Antigravity CLI, Loaf, and Zed read none there, so SkillBinder reaches them in a workspace only through `.agents/skills`, and Amp also through Claude Code's `.claude/skills`. Replit documents its own directory only for its skill picker, and Sarvam Code has no evidence.

Of the 12 agents whose registry global directory is shared, 9 also read one of their own: Amp, Cline, Codex (deprecated by Codex), Dexto (legacy), fx, Kimi Code CLI, Loaf, Pi, and Warp. Zed reads none, Replit documents none, and Sarvam Code has no evidence.

A directory named for one agent is not always read by that agent alone. Warp reads `.codex/skills`, `.cursor/skills`, `.gemini/skills`, `.github/skills`, `.factory/skills`, and `.opencode/skills`, and the same folders under home; Cursor, Firebender, and fx also read `.codex/skills`; Firebender also reads `.cursor/skills`; and fx also reads `.opencode/skills`. A copy in the own directory of Codex, Cursor, Droid, Gemini CLI, GitHub Copilot, or OpenCode therefore reaches Warp too.

### An own directory does not hide the shared copy

An agent that reads its own directory still reads the shared ones. When both hold the slug, Codex, fx, and Loaf keep both, which is the "loaded twice" case; so does Warp unless the copies are identical, and the Cursor IDE lists both without saying which one the agent uses. The others use one copy and ignore the other, and which one differs by agent: the shared copy for Amp, Cline, Deep Agents, Dexto, Gemini CLI, and Copilot in VS Code; the own copy for Kilo Code, Kimi Code CLI, Pi, and the Copilot CLI; no documented or fixed winner for Droid, Firebender, and OpenCode. Gemini CLI warns about the override at every session start, even when the copies are identical, and Pi warns about the collision; Kilo Code and OpenCode only log it.

Scopes differ too. Among agents that keep one copy, most prefer a workspace copy to a global one, but Amp prefers `~/.config/agents/skills`, `~/.agents/skills`, and `~/.config/amp/skills`, Cline prefers any global copy, Antigravity CLI prefers `~/.gemini/antigravity-cli/skills` and `~/.gemini/skills`, Deep Agents prefers `~/.claude/skills` to every workspace directory but `.claude/skills`, and Kilo Code prefers `~/.kilo/skills` even to a workspace `.kilo/skills` copy. A skill both active for your agents and in a workspace can therefore have its workspace copy ignored, which the "Also in your agents" badge does not say.

### More readers than the registry knows

`~/.agents/skills` is also read by nine agents whose registry global directory is another one: Amp, Cursor, Deep Agents, Droid, Firebender, Gemini CLI, GitHub Copilot, Kilo Code, and OpenCode. The app's universal agents are only the agents whose registry directory is `~/.agents/skills`, so a copy there reaches more agents than the app shows.

Agents also read other agents' directories. `.claude/skills` is read by Amp, Cline, Cursor, Deep Agents, Firebender, fx, GitHub Copilot, Kilo Code, OpenCode, Replit, and Warp; `.codex/skills` by Cursor, Firebender, fx, and Warp; `.cursor/skills` by Firebender and Warp; `.opencode/skills` by fx and Warp; `.gemini/skills`, `.github/skills`, and `.factory/skills` by Warp; Goose's `.goose/skills` by Firebender; OpenClaw's `skills` by Dexto and fx; `~/.gemini/skills` by Antigravity CLI and Warp; and `~/.config/opencode/skills` by fx. A copy activated for Claude Code alone in `.claude/skills` therefore reaches up to eleven more agents. The overlap warning pairs only each agent's registry directories, so it misses, for example, a skill held both in `~/.agents/skills` and in a workspace's `.claude/skills`, which Cursor and GitHub Copilot both read.

### When a workspace copy loads

SkillBinder writes at the top of a workspace folder, and not every agent reads there. Gemini CLI reads only the directory it starts in, as do Pi for `.pi/skills`, Codex outside a git repository, and Dexto outside a Dexto project. Inside a repository, the Cline CLI, Deep Agents, and Kimi Code CLI read only the repository root's directories, so they miss a workspace below it. Deep Agents, Loaf, and Warp read workspace skills only inside a git repository. Gemini CLI, Pi, and Zed load them only once the folder is trusted, and Pi's print, JSON, and RPC modes skip them by default.

### Registry entries the evidence questions

- Antigravity: the registry's global root, `~/.gemini/antigravity/skills`, is documented as legacy; the docs name `~/.gemini/config/skills`.
- Replit: no Replit source documents the registry's global root, `${XDG_CONFIG_HOME:-~/.config}/agents/skills`, or any other global directory.
- Deep Agents: the agent that reads these directories is `dcode` from `deepagents-code`, and its global directory moves with `DEEPAGENTS_HOME` and the agent name.
- Sarvam Code: no public source confirms either directory.

## Proposal

This is a proposal only; the registry is unchanged. If it is accepted, a follow-up issue makes the change and regenerates `docs/supported-agents.md`.

### Keep the registry byte-identical

`xtask/src/registry.rs` pins `REGISTRY_SHA256` over the whole of `fixtures/contracts/registry-79.json` and requires its `upstream` to be one repository at one commit, `vercel-labs/skills` at `13e4063a1cf913f5606d57d42ab83a86f5001e04`, with the hashes of `src/agents.ts` and `src/types.ts`. Upstream still models one directory per agent at `9197d919034d19010ae303cf96bf19ad087dd4be` (2026-11-03): since the pin, `src/agents.ts` changed only Reasonix's home folder, and `AgentConfig` in `src/types.ts` has no field for another directory. The findings come from each agent's own sources, which that single provenance cannot describe. Writing them into the registry would break its provenance, and refreshing it from upstream would stop being a mechanical step.

### Add `fixtures/contracts/registry-directories.json`

One entry per agent with evidence, or per surface where a product's surfaces differ, lists every skill directory the agent reads, including the registry's, so the order between them can be recorded:

```json
{
  "overlayVersion": 1,
  "registrySha256": "eb1365a541f5e95f3adc6bb5f3c0a1a2fb2f84be0d82ec942f50b669b004c044",
  "agents": [
    {
      "agent": "kimi-code-cli",
      "workspace": [
        { "path": ".kimi-code/skills", "role": "own" },
        { "path": ".agents/skills", "role": "shared" }
      ],
      "global": [
        { "path": "${KIMI_CODE_HOME:-~/.kimi-code}/skills", "role": "own" },
        { "path": "~/.agents/skills", "role": "shared" }
      ],
      "sameSlug": "precedence",
      "order": [[".kimi-code/skills"], [".agents/skills"], ["${KIMI_CODE_HOME:-~/.kimi-code}/skills"], ["~/.agents/skills"]],
      "sources": [
        {
          "url": "https://github.com/MoonshotAI/kimi-code/blob/2c73f3f50af4fa7f2540ca0d2e33b07a1408e3a7/packages/agent-core-v2/src/features/skill/catalog/skillRoots.ts#L8-L39",
          "revision": "2c73f3f50af4fa7f2540ca0d2e33b07a1408e3a7",
          "retrievedAt": "2026-11-04"
        }
      ]
    }
  ]
}
```

- `workspace` paths are relative to the workspace folder; `global` paths use the registry's `globalRoots` template syntax (`~`, `${VAR:-default}`, or an absolute path). Paths compare after expansion with every variable unset, so `${KIMI_CODE_HOME:-~/.kimi-code}/skills` and `~/.kimi-code/skills` are one directory.
- `role` is `own` for a directory named for this agent; `shared` for `.agents/skills`, `~/.agents/skills`, and `${XDG_CONFIG_HOME:-~/.config}/agents/skills`; `compatibility` for any other directory it reads, whether another agent's, such as `.claude/skills`, or a variant no registry agent uses, such as `.agent/skills`; and `unconfirmed` for a registry directory no source confirms, such as Replit's global root. A copy in a directory reaches every agent whose entry lists it, so an `own` directory is for one agent alone only where no other entry lists it; Warp, for one, lists Cursor's `.cursor/skills`.
- `surface` names the surface an entry describes where surfaces differ, such as GitHub Copilot's VS Code agent and CLI, which order `.agents/skills` and `.github/skills` oppositely. A directory no recorded surface reads is left out.
- `sameSlug` is one of the findings' four verdicts and covers every directory the entry lists, so it can differ from the summary's same-scope column: Zed is `precedence` here and "nothing to compare" there. `order` lists tiers, winner first, each holding the directories a source leaves unranked against each other, such as Droid's `.factory/skills` and `.agents/skills`; it is required with `precedence`, allowed with `one`, and omitted otherwise.
- Each source on a repository host has a full commit SHA, which its URL also names. Docs published on a website have a URL, `retrievedAt`, and an archived copy where one exists; they cannot be hash-pinned, so they are reviewed again with the overlay.
- `registrySha256` ties the overlay to the reviewed registry, so refreshing the registry forces a new review of the overlay.

`xtask` gets a `REGISTRY_DIRECTORIES_SHA256` constant and a step in `cargo run -p xtask -- verify` that rejects the overlay when its bytes or `registrySha256` change; an `agent` is not a registry id or has two entries for one `surface`; an entry leaves out one of the agent's registry directories or lists a path twice; a workspace path is absolute or leaves the folder; a global path does not start with `~`, `${`, or `/`; a `shared` path is not one of the three shared directories; `order` names a path the entry does not list; or a source lacks the SHA or the `retrievedAt` its kind requires. Agents with "no evidence found" stay out.

### What the follow-up must change

- `crates/engine/src/discovery/registry.rs:17`: `Agent` reads one `workspace_skills_dir`. Merge the overlay into the loaded agents. `locations()` (`:54`) then scans every directory, `workspace_skill_dirs()` (`:89`) returns all of them, and `agent_options()` (`:116`) passes them on. `global_roots` is already a list (OpenClaw has three); overlay directories go after the registry's roots, so `global_dir()` (`:142`), and with it detection and the global skill directory the app shows, stays the registry's first root until a separate decision changes it.
- `global_skill_dirs()` (`:98`) and `workspace_skill_dirs()` also decide which folders are agent folders (`is_agent_folder`, `crates/engine/src/workspaces/projects.rs:67`), which `crates/engine/src/workspaces/store.rs` refuses to add as workspaces (`:51`) and forgets at launch (`:316`). With the overlay, a folder inside `~/.codex/skills`, or a project whose `.codex/skills` links there, becomes one; the follow-up decides whether that is wanted.
- `crates/proto/src/workspaces.rs:44`: `AgentOption.workspace_skills_dir: String` keeps the registry directory and gains the agent's other directories with their roles, so a workspace can offer `.kimi-code/skills` for Kimi Code CLI.
- Its readers compare one directory: `overlaps` (`crates/engine/src/activation/targets.rs:117`), `detected_dirs` (`crates/engine/src/workspaces/activation.rs:114`), and `dir_rows`, the workspace agents menu (`crates/ui/src/native_view/workspace_targets.rs:39`).
- `overlaps` warns that an agent "may load it twice" for every overlap. With the overlay, an agent with `precedence` or `one` uses one copy, so only `both` and `unknown` need that warning; what `precedence` shows instead, that a copy elsewhere hides this one, is separate UI work.
- `xtask/src/registry.rs::document` gains a column for the other directories, and `docs/supported-agents.md` is regenerated.
- Fixtures that build `AgentOption` by hand: `crates/engine/src/activation/targets.rs:151`, and in `crates/ui/src/native_view/`, `agent_page.rs:169`, `agent_skills.rs:132`, `library_copies.rs:134`, `import_review.rs:80`, and `workspace_targets.rs:286`.
- `docs/agents/domain.md` defines an agent as "a registered skill reader with documented global roots and a direct workspace skill directory"; `docs/features/native/workspace-activation.md` describes the agents menu and the overlap warning.
