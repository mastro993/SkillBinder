# Deployments write skill copies into Root and projects

Until now SkillBinder only read skills outside its library. Imports copied files in and never changed their sources. Deployments change that: adding a skill to Root or to a project copies it into that target's agent skill directories, and removing it deletes that copy. The library copy stays in the catalogue.

We chose copies over symlinks. A symlink breaks when the library moves, and it does not survive a project's own Git history on another machine. Because the copies can drift from the library, each deployment records the digest it was written with. A rescan compares that digest with the library and with the project copy to tell which side changed. SkillBinder never resolves drift silently; the user picks the remedy.

Only explicit deployment actions write outside the library, and only to the paths a deployment records. Import and Discovery stay read-only. A library skill that still has deployments cannot be deleted. Removing a drifted copy asks for confirmation first, because it discards edits made in the project.

## Considered options

- Symlinks into the library: no drift, but fragile across machines and library moves.
- Record-only deployments with no writes: safe, but users would still manage project skill folders by hand.
