# 10 — Install skills from public GitHub sources

**What to build:** Let the user enter a supported public GitHub reference, inspect inert repository content at an exact commit, select valid skills, and install them into the canonical library with provenance but without deployment.

**Blocked by:** 03 — Import one local skill safely.

**Status:** ready-for-agent

- [ ] One parser accepts the approved GitHub owner/repository, HTTPS, SSH, tree URL, and restricted `skills add` forms and rejects arbitrary commands, unsupported providers, insecure transports, embedded secrets, and local paths.
- [ ] Normalization separates provider, repository, ref kind/value, selected commit, skill path, and optional requested skill without other modules reparsing user text.
- [ ] Branch names containing slashes and tree paths are resolved against repository refs instead of guessed by string splitting.
- [ ] Retrieval uses supervised system Git in an app-owned isolated repository with approved configuration and no repository hooks, filters, checkout scripts, remote helpers, or general shell surface.
- [ ] Repository inspection walks inert Git objects, enforces tree/cache/payload limits, and materializes only selected validated skills.
- [ ] Preview shows repository identity, exact commit, detected skill paths, descriptions, validation, file counts, sizes, executable files, applicable license information, and warnings.
- [ ] The user explicitly selects skills; SkillBinder never installs every detected folder by default.
- [ ] Install adds complete canonical payload, source provenance, accepted commit/digest, and portable metadata but never deploys or automatically commits it.
- [ ] A moved ref, changed repository, cancellation, limit, or retrieval failure invalidates preview and leaves the library unchanged.
- [ ] Unit tests cover every input form, ref ambiguity, transport policy, supervised Git requests, inert tree selection, limits, preview mapping, provenance, no-deployment behavior, and failure atomicity through fake ports.

