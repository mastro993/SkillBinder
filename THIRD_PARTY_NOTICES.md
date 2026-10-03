# Third-party notices

SkillBinder depends on third-party Rust packages under their respective licenses. `Cargo.lock` identifies the exact graph. Imported skills and their payloads retain their source licenses.

## Native UI dependencies

- Ely GPUI Components, revision `2f8b2f687cd1e9b98a7cd29d4e882d09406fc547`, is licensed under MIT or Apache-2.0. Its embedded font assets retain their accompanying licenses.
- GPUI and its platform crates, official revision `1a28cff4b409169bac058bca40dfbfeb7621d19b`, retain their upstream license terms.
- Hugeicons artwork is extracted from `@hugeicons/core-free-icons` version `4.3.5`. The complete MIT notice is in `assets/hugeicons/LICENSE.md`. `inventory.json` records the exact package modules, artwork hashes, and variants.

The historical font license records under `fixtures/contracts/font-licenses` document baseline research. No font binaries from the previous application are bundled.

## vercel-labs/skills registry snapshot

- Project: `vercel-labs/skills`
- Commit: `7407f3893ad4dceab546ac002c3ef806e4000c73`
- License: MIT
- Files: `src/agents.ts`, `src/types.ts`
- Use: reviewed declarative registry data with pinned source hashes; upstream executable source is not bundled. The native gate verifies preserved bytes and generated documentation, not a fresh comparison against upstream.
- Copyright: Copyright (c) 2026 Vercel, Inc.
