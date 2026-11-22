# The command line tool covers what changes the library, and asks the way the app asks

The user asked for a command that does everything the window does. The window does two kinds of things: it changes or reads the library and what is active where, and it arranges itself (sidebar width, theme, folded sections, preset and workspace colors, icons and images, ordering, which page and dialog are open). Only the first kind means anything outside the window. So the command covers the domain actions: reading the library, presets, library folders, workspaces, your agents, managed copies, scan and import, Explore's search and install, your repositories, skill updates and Reinstall, Sync, Push, Pull, and the sync remote ([spec](../specs/command-line.md)). It leaves out appearance and ordering, onboarding, the app's own settings, app updates, and Move to Applications. Deleting a skill from the library stays out, as in the app (ADR 0007).

The command serves people and coding agents alike. It prints readable text by default, and with `--json` one JSON document per command on stdout, in a shape documented per command, with progress on stderr. Exit codes are stable: 0 done, 1 failed, 2 usage, 3 waiting for the user's answer, 4 SkillBinder busy, 5 SkillBinder unavailable or not set up.

The app's confirmations keep their meaning. A same-slug copy SkillBinder does not manage is replaced only after the user confirms (ADR 0006), and an import states each replacement before it runs (ADR 0008). In a terminal the command asks the same question the app asks. Without a terminal it never asks and never replaces: the copies wait under "need attention", as after the app's Done pill, and the command exits with 3, unless the caller passed `--replace`, which is the user's confirmation. Deleting a preset or a library folder and removing a workspace ask the same way and take `--yes`.

## Considered Options

- Literal parity with the window, appearance and ordering included, was rejected by the user. Every future change to the interface would need a matching command, and nobody in a terminal gains from setting a preset's color.
- A smaller first cut of read-only commands, preset switching, and install was rejected by the user in favor of all the domain actions, delivered in slices.
- Never replacing a copy from the command was rejected by the user. A script could then never finish a change that meets a copy, and the user would have to open the app.
- Confirming only at a terminal, with no flag, was rejected by the user for the same reason: agents could never act on the user's explicit instruction.
- JSON by default, or readable text only, was rejected by the user: both people and agents use it.

## Consequences

- Before onboarding finishes, every command but `status` refuses and says to open SkillBinder to finish setting up.
- An agent passing `--replace` overwrites the user's own copies. The flag is never implied and its help says so.
- The JSON shapes and exit codes are a contract: changing one breaks scripts, so a change to a shape adds a field or a new command.
