# App updates design reference

`prototypes/settings-updates.html` is the prototype for Settings › Updates and the download-only notice of app updates slice S1 (#169; [spec](../../specs/app-updates.md) rules 9, 13, 18, 19). The user picked variant A "Inline" on 2026-11-20: the status line with its action at the row's end, as the Git row places Install. B "Stacked" and C "Under the list" are kept for reference. The muted line of the notice and of the Status row in S1 is "Download it from the release page.", since the reasons of rule 13 come with the slices that can tell them.

Open the bundle in a browser. Its demo controls pick the build (0.1.0 or a beta), the state, the download-only reason of later slices, and whether the new agent notice and the progress strip show; `?state=available&variant=b` opens on a state. The prototype is guidance for layout, hierarchy, copy, and behavior, not pixel parity: the app builds it from GPUI Kit and the design system.

`source/settings-updates.html` is its source, built on the shared kit:

```sh
python3 -I docs/design/scoped-pages/source/kit/inline.py docs/design/app-updates/source/settings-updates.html docs/design/app-updates/prototypes/settings-updates.html
```
