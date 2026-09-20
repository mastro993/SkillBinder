# 11 — Add supported skills.sh catalog discovery

**What to build:** Provide first-class in-app skills.sh discovery and installation only when a documented stable integration exists, otherwise stop at a visible integration gate without scraping or inventing an API.

**Blocked by:** 10 — Install skills from public GitHub sources.

**Status:** ready-for-agent

- [ ] Implementation records the reviewed skills.sh integration contract, authentication needs, stability status, and evidence before enabling catalog discovery.
- [ ] When a supported interface exists, the user can search or browse catalog results inside SkillBinder and see source owner, repository, skill selector, description, and available revision context.
- [ ] Selecting a catalog result resolves to the existing normalized GitHub source request and reuses exact-commit preview, validation, selection, provenance, and canonical installation behavior.
- [ ] A skills.sh page reference resolves the requested skill against repository contents and requires a choice when several paths match.
- [ ] Catalog results never bypass source ownership, license, executable-file, plugin-manifest, limit, or trust review.
- [ ] Installing from catalog remains separate from folder/tag assignment impact, explicit Commit, deployment, source update, and sync.
- [ ] When no suitable documented stable interface exists, the feature shows a clear unavailable state and implementation stops at that gate.
- [ ] SkillBinder does not scrape HTML, copy a Vercel project token, execute the skills CLI, or call an undocumented catalog API.
- [ ] Unit tests cover supported result mapping, ambiguous selectors, unavailable-gate behavior, authentication/error mapping, reuse of GitHub preview, and prohibition of undocumented fallbacks through fake catalog and source ports.

