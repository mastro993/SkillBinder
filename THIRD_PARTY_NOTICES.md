# Third-party notices

SkillBinder depends on third-party Rust packages under their respective licenses. Lockfiles identify the exact dependency graph. This repository does not relicense imported skills or their payloads; source and license provenance must remain attached when installation support is implemented.

## vercel-labs/skills registry snapshot

- Project: `vercel-labs/skills`
- Commit: `7407f3893ad4dceab546ac002c3ef806e4000c73`
- License: MIT
- Files: `src/agents.ts`, `src/types.ts`
- Use: reviewed declarative registry data and comparison fixture only; upstream JavaScript is not bundled or executed.
- Copyright: Copyright (c) 2026 Vercel, Inc.

## Native UI

GPUI and gpui-component are Apache-2.0 licensed. Cargo.lock records exact versions.
The Hugeicons SVGs bundled under `crates/ui/assets/icons` were extracted from the previously used
`@hugeicons/core-free-icons` package. They are MIT licensed, Copyright (c) 2025 Hugeicons;
the full license is retained next to the assets. No icons are downloaded at runtime.
