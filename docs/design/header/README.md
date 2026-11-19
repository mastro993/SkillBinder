# Window shell design reference

This folder holds the prototype the user picked a window shell from on 2026-11-19 ([ADR 0016](../../adr/0016-split-columns-shell.md)). The sizes and colors are in `DESIGN.md` § Layout; the behavior is in [the application shell](../../features/native/application-shell-continued.md). Build it with GPUI Kit components and the design system's components first, styled through their public API. Small visual differences from Kit's defaults are fine.

## Chosen variant

| Area | Prototype | Variant the user picked |
|---|---|---|
| Window shell and page header | `prototypes/header.html` | D "Copilot" |

- **D "Copilot":** after the GitHub Copilot desktop app. No page panel: the sidebar on its tinted canvas and the page on the page color are two flat columns split by one `border` line. The sidebar's 52-point top row holds the traffic lights and the sidebar toggle on the left and Back and Forward on the right. The page's 52-point header sits on the page color: the title (Library's folder tabs, or a place's glyph, name, a dot, and its folder in mono, muted, truncating), the actions on the right, and a `border` line under it. With the sidebar hidden, the toggle, Back, and Forward sit together after the lights.

The rejected variants stay in the bundle: A "Clustered" kept the header on the canvas over the inset panel with the window controls grouped after the lights; B "Chrome strip" held only the window controls in a 40-point strip and moved the title into a toolbar inside the panel; C "Two panels" also raised the sidebar into a panel.

Open the bundle in a browser. Its bar switches the variant, the theme, the page (Library, a workspace with a long folder, an agent), the sidebar (open or hidden), and the history (no Back, Back only, both). "Current" shows the shell before the change.
