# The user chooses which agents SkillBinder manages

SkillBinder manages every detected agent: it scans its global skill directory, and presets that hold a skill for all your agents write copies there. Some users keep an agent they do not want touched. So the user chooses agents in onboarding and later in Settings, and only chosen agents are your agents: SkillBinder reads and writes skill directories only for them. An agent that is detected but not chosen is not scanned, has no sidebar row or page, receives no copies, and adds no workspace skill directory.

The choice is made per global skill directory, not per agent, because what is activated is a skill directory (ADR 0006): every agent that reads `~/.agents/skills` loads what is there, so the universal agents are one choice, as they are one Universal row in the sidebar. The choice is saved as the set of chosen global skill directories, and an agent detected later whose directory is already chosen is chosen with it.

All your agents now means every chosen agent's global skill directory, including agents chosen later; a preset that names specific agents keeps an unchosen one in its list but writes nothing for it. A newly detected agent whose directory is not chosen is asked about once, with a notice offering to manage it; either answer is remembered. Choosing an agent applies the active preset to it like any change that writes copies, and its copies with the library's content are adopted. Turning an agent off leaves its copies in place, unmanaged, as removing a workspace does (ADR 0006): it keeps loading them, and SkillBinder no longer updates or removes them.

## Considered Options

- Managing every detected agent, as before, was rejected. It writes into agents the user wants left alone.
- A choice per agent was rejected. Agents that share a directory would offer a choice that does not exist, as ADR 0007 already found for toggles.
- Managing a newly detected agent automatically was rejected. It overrides the user's choice of agents the moment they install one.
- Leaving a newly detected agent off without asking was rejected. The user would rarely notice that a new agent exists.
- Removing the copies when an agent is turned off was rejected. It changes what the agent loads as a side effect of telling SkillBinder to leave it alone; the user can remove skills before turning it off.

## Consequences

- Detection still decides which agents can be chosen: an agent whose directory does not exist is not offered.
- Found skills in unchosen agents' directories are not offered for import.
- Specific agent lists in presets and workspaces can name an unchosen agent; they apply again when it is chosen again.
