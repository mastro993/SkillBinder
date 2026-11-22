# The SkillBinder skill lives outside the library

Agents connected to the MCP server (ADR 0025) see about fifteen tools but no guide to using them well: which to read first, that `install_skills` leaves a skill inactive, when to ask the user before `replace`. SkillBinder now gives connected agents its own skill, the **SkillBinder skill** ([spec](../specs/command-line/onboarding.md#the-skillbinder-skill)). This reverses decision D7 of the command line tool spec, which left a skill for agents to a later feature.

The user wants it to survive every configuration, to stay out of sight inside SkillBinder, to count as no skill installed, and to be removable only from Settings. A library skill can do none of that: presets decide where it is active (ADR 0007), Library lists and counts it, and Sync carries it to other devices (ADR 0004). So it is not a library skill. It is SkillBinder's own, `skills/skillbinder/` in SkillBinder's repository, compiled into the app, and it changes only with an app update.

SkillBinder writes it as a managed copy (ADR 0006), through the same copy records, adoption, and conflict rules as a library skill's copies, into the global skill directory of each chosen agent (ADR 0012) that is connected to the MCP server (ADR 0026), while one device setting, **Teach connected agents**, is on. Its place is computed from those facts alone, never from the active preset or a workspace, so switching presets, editing them, or adding workspaces never touches it. Connecting an agent adds it there; disconnecting, or turning the setting off in Settings › Agents › MCP server, removes it. A same-slug copy SkillBinder does not manage is a conflict the user confirms, as ADR 0006 requires. An app update rewrites the managed copies whose content differs; one edited in place asks first, like any managed copy.

It is invisible elsewhere. Scans skip every `skillbinder` folder, so it is never a found skill, in the Import skills dialog, or in Scan for skills' count. Import and install refuse the slug as reserved. Library, Agents, workspace pages, Explore, the command's `skills`, and the MCP server's `list_skills` never list it, and no count includes it. It appears only in its Settings row and in `status`. The setting and its copy records are machine-local and never sync; each device offers it in its own onboarding or Settings.

## Considered Options

- A library skill in every preset was rejected by the user. It would show in Library and counts, sync to devices that never asked for it, and be removable from any preset.
- A library skill held "always on" across presets was rejected. It would add a rule that new presets are not empty and still show in Library.
- Writing a plain file into the agents' folders was rejected. SkillBinder writes only managed copies there (ADR 0006), and an unmanaged file would turn up in scans as a found skill.
- Installing it from the catalog with Git was rejected. Git is optional (ADR 0011), and the skill must match the app's MCP tools, which only the app's own version guarantees.
- A skill per agent chosen in Settings was rejected in favor of following the MCP connection: the skill only helps an agent that has the tools.

## Consequences

- ADR 0006's managed copies now include the SkillBinder skill beside library skills. Its copy records carry an origin, library or app, so activation passes, the review list, and Retry handle both.
- The slug `skillbinder` is reserved. A library that already holds a skill with that slug keeps it, and on that device the SkillBinder skill is not offered.
- Agents whose global skill directory is shared, such as `~/.agents/skills`, all see the skill once any agent reading it is connected.
- The skill's text is part of the MCP contract: a change to the tools changes it in the same release.
