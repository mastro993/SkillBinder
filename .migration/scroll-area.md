# scroll-area

2026-09-20. Transformation engine, since the `new-york` style has no base counterpart and the app's own class names and CSS had to survive. Migrated.

## Changed

`apps/frontend/src/components/ui/scroll-area.tsx`

- Imports `ScrollArea` from `@base-ui/react/scroll-area`.
- Added the `ScrollArea.Content` part between the viewport and the children. Base UI observes that element to recompute thumb geometry, so children cannot sit directly in the viewport.
- Dropped the `type = "auto"` default and its pass-through. Base UI has no such prop and mounts a scrollbar only while its axis overflows. No caller passed it.
- Kept the app's `scroll-area*` class names, the `forwardRef` shape, and the `ScrollArea` and `ScrollBar` exports, so no caller needed an edit.

`apps/frontend/src/app/styles/index.css:407`

- The viewport child pin moved from `[data-radix-scroll-area-viewport] > div` to `.scroll-area-content`, same `min-width: 0`. Base UI sets `min-width: fit-content` inline on Content. Without the pin, the discovery table stretched its own card from 576px to 636px at a 700px window instead of scrolling inside it. Measured both ways, numbers under Verify by hand.
- Dropped `display: block !important`. Radix rendered the viewport child as a table, which is what the old rule existed for. Content is a plain div.

`apps/frontend/src/test/setup.ts:17`

- Stubbed `Element.prototype.getAnimations`. Base UI's viewport asks for subtree animations on its scroll-end timeout, jsdom has no such method, and the callback threw after teardown. Before the stub, `vitest` exited 1 with 6 unhandled errors while its 18 tests passed.
- The stub uses `Object.defineProperty`, matching the ResizeObserver stub above it. A direct assignment does not compile, because TypeScript narrows `Element.prototype` to `never` inside the `in` guard for a method that lib.dom already declares.

`apps/frontend/package.json`, `pnpm-lock.yaml`

- `@base-ui/react` 1.8.0 added and `@radix-ui/react-scroll-area` 1.2.18 removed in the same change. `grep -c radix-ui pnpm-lock.yaml` returns 0.

`docs/dependencies.md:18`, `docs/mvp-technical-specification.md:226`

- The primitive row and the styling row now name Base UI.

Leftover scan, run over this component's files, the rest of `apps/frontend`, and the docs:

```
$ grep -rn "radix-ui\|@radix-ui" apps/frontend/src docs apps/frontend/package.json components.json
no matches
```

## Left alone

- `apps/frontend/components.json` still reads `"style": "new-york"`, which is now wrong about the primitive base. It is a legacy style with no `base-*` counterpart, so flipping it would restyle every component. Flagged for a decision, not fixed. Until then `shadcn add` keeps delivering Radix variants.
- The three consumers pass only `className`, so none changed. `components/layout/app-shell.tsx:39`, `features/discovery/components/views/discovery-candidate-table.tsx:23`, `features/onboarding/components/onboarding-view.tsx:61`.
- This project has no cmdk, vaul, sonner, input-otp, react-day-picker, or recharts wrapper, so the exclusion list did not apply.

## Behavior changes

- The wrapper API lost `type`. Radix's `"auto"` default and the pass-through are gone, and passing `type` now fails typecheck. No caller passed it.
- No runtime difference was observable. Default-window geometry, viewport scroll extents, scrollbar rects, thumb rects, and corner presence are identical across `/discovery`, `/library`, and `/onboarding`, and identical at a 700px window on `/discovery`.

## Verify by hand

Compared on the fixture server, before and after, from a DOM probe that records each scroll area's rect, its viewport client and scroll extents, its content rect, and its scrollbar and thumb rects.

1. Default window, 1706x960, on `/discovery`, `/library`, and `/onboarding`. Ran, identical. The probe's JSON diffs clean.
2. Narrow window, 700x600, on `/discovery`. Ran, identical. The candidate card scrolls sideways inside itself, `document.body.scrollWidth` stays 700, and the main content keeps its vertical thumb of 190px against 1887px of content. Before the `.scroll-area-content` pin, this case failed with the card stretched to 636px and no horizontal scrollbar inside it.
3. Narrow window on `/library` and `/onboarding`. Not run. Both render a single non-overflowing scroll area, so the pin cannot apply to them.
4. Keyboard. Tab into the discovery table, then scroll the viewport with arrow keys. Not run. Worth a minute before merge, because the scroll container is now the viewport element rather than a Radix-managed child.
