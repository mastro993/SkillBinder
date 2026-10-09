# Third-party notices

SkillBinder depends on third-party Rust packages under their respective licenses. `Cargo.lock` identifies the exact graph. Imported skills and their payloads retain their source licenses.

## Native UI dependencies

- GPUI Kit 0.7.0 and its GPUI Base and GPUI Component crates are licensed under Apache-2.0. Their source is available at `https://github.com/longbridge/gpui-kit`, release revision `0c830f4d257e69fdd17200650533ab4ca9a40cc0`.
- GPUI Pre 0.3.7 and its platform crates retain their upstream license terms. The shared backend is supplied by the unmodified `gpui-mcp` repository at revision `9dda8e5cb49990261e3fdaa26abe38112d30dafb`; its GPUI patch and Apache-2.0 MCP bridge retain their notices. The bridge is included only in native-test builds.
- Hugeicons artwork is extracted from `@hugeicons/core-free-icons` version `4.3.5`. The complete MIT notice is in `assets/hugeicons/LICENSE.md`. `inventory.json` records the exact package modules, artwork hashes, and variants.

The historical font license records under `fixtures/contracts/font-licenses` document baseline research. No font binaries from the previous application are bundled.

## vercel-labs/skills registry snapshot

- Project: `vercel-labs/skills`
- Commit: `7407f3893ad4dceab546ac002c3ef806e4000c73`
- License: MIT
- Files: `src/agents.ts`, `src/types.ts`
- Use: reviewed declarative registry data with pinned source hashes; upstream executable source is not bundled. The native gate verifies preserved bytes and generated documentation, not a fresh comparison against upstream.
- Copyright: Copyright (c) 2026 Vercel, Inc.
