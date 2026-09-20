# project

2026-09-20. Whole-project Radix to Base UI migration. One Radix wrapper existed, so this run migrated the entire Radix surface.

## Dependency swap

- Added `@base-ui/react` 1.8.0.
- Removed `@radix-ui/react-scroll-area` 1.2.18 in the same change. `grep -c radix-ui pnpm-lock.yaml` returns 0.
- `apps/frontend/components.json` still reads `"style": "new-york"`. Legacy styles have no `base-*` counterpart, so the CLI cannot be pointed at Base UI without restyling every component. Flagged, not fixed. Until it changes, `shadcn add` delivers Radix variants and the file lies about the primitive base.

## App-code sweep

Three call sites, all passing only `className`, none needing a prop change.

- `apps/frontend/src/components/layout/app-shell.tsx:39`
- `apps/frontend/src/features/discovery/components/views/discovery-candidate-table.tsx:23`
- `apps/frontend/src/features/onboarding/components/onboarding-view.tsx:61`

No `asChild` usage, no Radix part imported outside the wrapper, no Radix portal or primitive reached from feature code. One wrapper file carried the whole migration.

## Final build

`pnpm verify` exits 0 after the migration. It runs `oxfmt --check`, `oxlint`, `tsc -b`, the IPC contract check, 18 frontend tests across 8 files, the architecture check, the registry check, and the Rust workspace tests. `pnpm --filter @skillbinder/frontend build` passes in 956ms, with `scroll-area` as its own 18.97kB chunk.

No test covers the scroll geometry. jsdom reports no layout, so the pin in `index.css` and the scroll bar mounts were verified in a real browser against the fixture server instead, with the numbers recorded in `scroll-area.md`.

Wrappers remaining on Radix, derived from disk: 0.
