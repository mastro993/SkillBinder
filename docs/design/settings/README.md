# Settings design reference

`prototypes/header-tabs.html` is the prototype for Settings' header tabs, which replaced the "One page" layout below on 2026-11-26 so Settings reads like Library. The user picked variant B "Search in header": the header holds "All", a separator, and a tab per section, with the search at its end and no page toolbar; All labels each section with its glyph and name, small and muted. A "Library toolbar" (the search in a page toolbar, sections keeping their title and line) and C "Card per section" (each section one card, its groups as subheads inside) are kept for reference. Its demo controls pick the Git and update states, a long library path, and opening from the new agent notice; `?variant=f&tab=general&q=sync` opens on a variant, a tab, and a search. Its source is `source/header-tabs.html`:

```sh
python3 -I docs/design/scoped-pages/source/kit/inline.py docs/design/settings/source/header-tabs.html docs/design/settings/prototypes/header-tabs.html
```

The rest of this page describes the earlier prototype, whose control rules still hold.

`prototypes/settings.html` is the prototype for the Settings redesign: sections, a search field, and one control per kind of setting. The user picked variant B "One page" on 2026-11-21: every section on one scrolling page, with the search and a button per section staying above it. A "Section list" (Kit's `Settings`, one section beside a section column), C "Tabs", and D "Dialog" (A inside a large dialog over the current page) are kept for reference.

The control rules the prototype's "Control rules" checkbox lists hold for every row:

| Setting | Control |
|---|---|
| On or off | switch |
| One of two | segmented control |
| One of three or more | select |
| An action | button |
| A fact the user can't change | muted text at the row's end |

Open the bundle in a browser. Its demo controls pick the Git and update states; `?variant=b&q=check&section=updates` opens on a variant, a search, and a section. The prototype is guidance for layout, hierarchy, copy, and behavior, not pixel parity: the app builds it from GPUI Kit and the design system. Behavior is in [Settings](../../features/native/settings-agents.md).

`source/settings.html` is its source, built on the shared kit:

```sh
python3 -I docs/design/scoped-pages/source/kit/inline.py docs/design/settings/source/settings.html docs/design/settings/prototypes/settings.html
```
