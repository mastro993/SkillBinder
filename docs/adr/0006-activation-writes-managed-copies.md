# Activation writes managed copies; presets are for your agents only

Activating a skill now copies it into the skill directories that read it, and deactivating removes the copy, so what the user chooses is what agents load. This reverses the rule that SkillBinder never writes into the agents' global skill directories or workspaces. It writes only managed copies: copies it wrote, or adopted because their content matched the library. It rewrites them when the library content changes and never touches any other file. A copy of the slug with other content is a conflict the user resolves, and a managed copy edited in place is never overwritten or removed without asking. Copies are full copies, not links, and are machine-local; only the library and records sync.

What the user activates is a skill directory, not an agent, because every agent that reads a shared directory such as `.agents/skills` loads what is in it. Presets apply only to your agents' global skill directories: one preset is active at a time, and it maps each skill to all your agents or to specific ones, so switching presets swaps the global copies. A project is already a scope, so each workspace holds its own skills, each for all of the workspace's skill directories or for specific ones, with no presets.

## Considered Options

- One preset mapping every scope, global and each workspace, was rejected. Switching it rewrites every directory on the machine, and workspaces cannot share or differ independently.
- Giving each skill directory its own preset, with presets as plain skill sets, was rejected. Making one agent differ in one project needs a new preset for every combination.
- Letting each workspace choose a preset was rejected for now. It shares a setup between projects, but a one-off skill in one project then means editing a shared preset or cloning it, and every toggle becomes ambiguous between the preset and the workspace.
- Symbolic links were rejected. Some agents do not follow them, and they break when the library moves.
- Rolling back a partly applied change was rejected. The choice is saved first and files are written after it, so the app always shows what the user chose and where it is not applied yet.

## Consequences

- Presence is retired; the Agents view marks which found skills are managed copies.
- An agent that reads several directories, or a skill active both for your agents and in a workspace, can load the same skill twice. The app warns instead of blocking.
- Activating for a single agent is possible only where that agent has a skill directory of its own. Agent-specific workspace directories beyond the registry's one per agent need evidence before they are added.
