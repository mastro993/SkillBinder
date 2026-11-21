# Settings design reference

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
