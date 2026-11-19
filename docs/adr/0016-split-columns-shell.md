# The window is two flat columns split by a line, not a page panel inset on a canvas

Partly supersedes ADR 0013: its look keeps the tinted sidebar canvas, the iris accent, and the elevation levels for selections, menus, popovers, and dialogs, but the page no longer sits on a raised panel inset from the window.

The header did not hold together. The sidebar toggle sat beside the traffic lights while Back and Forward sat far from it at the sidebar's edge, the page title and its actions sat on the canvas while the page sat on the panel below them, and controls on the canvas lost contrast: a disabled chevron nearly vanished and the chosen folder tab barely showed in light mode. A long workspace folder also pushed the page's actions out of the window.

So the window follows the GitHub Copilot desktop app's shell. It is two flat columns: the sidebar on its tinted canvas and the page on the page color, split by a 1-point `border` line on the sidebar's trailing edge. Each column starts with the 52-point header row. Over the sidebar it holds the traffic lights and the toggle beside them, with Back and Forward at its trailing edge; while the sidebar is hidden, the toggle, Back, and Forward sit together after the lights. Over the page it holds the page's title and its actions on the page color, with a `border` line under it. A place's title is its name, a dot, and its folder in mono, and it truncates rather than pushing the actions out. The layout is in `DESIGN.md` § Layout, and the chosen prototype is [docs/design/header](../design/header/README.md).

## Considered Options

- Clustering the toggle, Back, and Forward after the lights and keeping the header on the canvas over the panel was rejected. It fixes the spacing but keeps the title and actions on a different surface from the page.
- A thin strip holding only the window controls, with each page's title and actions in a toolbar inside the panel, was rejected. It keeps the panel's inset and shadow for little gain over a flat column.
- Raising the sidebar into a panel of its own as well was rejected. It changes the most for the same result, and the sidebar's selection would lose its raised pill.

## Consequences

- `DESIGN.md` drops the `raised-panel` shadow, and the design system drops `Surface::panel` and `page::INSET`. The header is 52 points instead of 44.
- The scoped pages' head, approved in the scoped-pages prototype, gains a dot before the folder and sets the folder in mono.
- Back and Forward are reachable while the sidebar is hidden.
