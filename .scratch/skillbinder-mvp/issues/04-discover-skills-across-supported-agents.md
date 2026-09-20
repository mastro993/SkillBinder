# 04 — Discover skills across supported agents

**What to build:** Show the user skills found in known global locations and explicitly selected project roots for every agent in the pinned supported registry, then let the user choose which results to import through the established safe import flow.

**Blocked by:** 03 — Import one local skill safely.

**Status:** ready-for-agent

- [ ] SkillBinder ships a declarative agent registry pinned to an upstream commit and digest, with every upstream agent ID represented or acceptance failing.
- [ ] Each registry rule records supported operating systems, available scopes, aliases, reserved or read-only locations, preferred deployment location, source evidence, review date, and distinct documentation/path/runtime test status.
- [ ] Onboarding and Discovery explain and inspect known global locations without requiring unrestricted home-directory or whole-disk access.
- [ ] The user can add, edit, disable, and remove project-search roots and custom skill locations through explicit grants.
- [ ] Project scanning is bounded, cancellable, traverses supported nested projects and worktrees, applies visible exclusions, and never treats a missing default root as permission to broaden scope.
- [ ] Unreadable directories, reached limits, invalid candidates, and unavailable roots appear as partial result entries rather than aborting the full scan.
- [ ] Results are paged and show location, reader agents, candidate name, link state, validation, size, duplicate state, and warnings before any import.
- [ ] Shared physical paths are scanned and displayed once with all known reader agents; agent names alone never create duplicate locations.
- [ ] Discovery remains read-only until the user selects candidates and approves their import; imported originals remain unchanged.
- [ ] Generated support documentation and unit fixtures cover every registry ID, applicable scope, alias, shared path, read-only path, custom-root behavior, scan boundary, and missing-scope case.

