---
version: alpha
name: SkillBinder
description: Inset canvas. A calm native macOS tool where content floats as a raised panel on a tinted window, with one iris accent for selection, primary actions, and focus.
colors:
  background: "#FFFFFF"
  foreground: "#1C1C21"
  sidebar: "#EDEDF0"
  sidebar-foreground: "#3A3A42"
  sidebar-accent: "#FFFFFF"
  muted: "#F1F1F4"
  muted-foreground: "#6B6B74"
  border: "#E6E6EA"
  hairline: "#EFEFF2"
  popover: "#FFFFFF"
  ring: "#5B5BD6"
  primary: "#5B5BD6"
  primary-hover: "#5151CD"
  primary-active: "#4747C2"
  primary-foreground: "#FFFFFF"
  accent-soft: "#EEEEFC"
  accent-soft-foreground: "#4746B8"
  success: "#2B9A66"
  success-text: "#18794E"
  warning: "#FFC53D"
  warning-text: "#A35A00"
  danger: "#CE2C31"
  danger-text: "#CE2C31"
  info: "#0797B9"
  info-text: "#0B7491"
  background-dark: "#1B1B1F"
  foreground-dark: "#EDEDEF"
  sidebar-dark: "#111113"
  sidebar-foreground-dark: "#B5B5BE"
  sidebar-accent-dark: "#2A2A30"
  muted-dark: "#2A2A30"
  muted-foreground-dark: "#A0A0AB"
  border-dark: "#2E2E35"
  hairline-dark: "#26262B"
  popover-dark: "#222227"
  ring-dark: "#6E6ADE"
  primary-dark: "#5B5BD6"
  primary-hover-dark: "#6E6ADE"
  primary-active-dark: "#7D78E5"
  accent-soft-dark: "#2A2A4A"
  accent-soft-foreground-dark: "#B1A9FF"
  success-text-dark: "#3DD68C"
  warning-text-dark: "#FFCA16"
  danger-text-dark: "#FF6369"
  info-text-dark: "#4CCCE6"
  preset-gray: "#737373"
  preset-red: "#DC2626"
  preset-orange: "#EA580C"
  preset-yellow: "#CA8A04"
  preset-green: "#16A34A"
  preset-blue: "#2563EB"
  preset-purple: "#9333EA"
  preset-pink: "#DB2777"
  preset-gray-dark: "#A3A3A3"
  preset-red-dark: "#F87171"
  preset-orange-dark: "#FB923C"
  preset-yellow-dark: "#FACC15"
  preset-green-dark: "#4ADE80"
  preset-blue-dark: "#60A5FA"
  preset-purple-dark: "#C084FC"
  preset-pink-dark: "#F472B6"
typography:
  title-lg:
    fontFamily: SF Pro
    fontSize: 20px
    fontWeight: 600
    lineHeight: 1.3
  title-md:
    fontFamily: SF Pro
    fontSize: 16px
    fontWeight: 600
    lineHeight: 1.4
  title-sm:
    fontFamily: SF Pro
    fontSize: 14px
    fontWeight: 600
    lineHeight: 1.4
  body-md:
    fontFamily: SF Pro
    fontSize: 14px
    fontWeight: 400
    lineHeight: 1.4
  body-sm:
    fontFamily: SF Pro
    fontSize: 13px
    fontWeight: 400
    lineHeight: 1.4
  label-md:
    fontFamily: SF Pro
    fontSize: 13px
    fontWeight: 520
    lineHeight: 1.4
  label-sm:
    fontFamily: SF Pro
    fontSize: 12px
    fontWeight: 520
    lineHeight: 1.4
  caption:
    fontFamily: SF Pro
    fontSize: 12px
    fontWeight: 400
    lineHeight: 1.4
  caption-sm:
    fontFamily: SF Pro
    fontSize: 11px
    fontWeight: 520
    lineHeight: 1.4
  micro:
    fontFamily: SF Pro
    fontSize: 10px
    fontWeight: 600
    lineHeight: 1.2
  mono-sm:
    fontFamily: SF Mono
    fontSize: 12px
    fontWeight: 400
    lineHeight: 1.4
rounded:
  xs: 4px
  sm: 6px
  md: 8px
  lg: 10px
  xl: 12px
  full: 9999px
spacing:
  2xs: 2px
  xs: 4px
  sm: 6px
  md: 8px
  lg: 12px
  xl: 16px
  2xl: 24px
  3xl: 32px
  control-sm: 24px
  control-md: 32px
  sidebar-row: 32px
  list-row: 56px
  header: 44px
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.primary-foreground}"
    typography: "{typography.label-md}"
    rounded: "{rounded.sm}"
    padding: 0 12px
    height: 32px
  button-primary-hover:
    backgroundColor: "{colors.primary-hover}"
    textColor: "{colors.primary-foreground}"
  button-primary-active:
    backgroundColor: "{colors.primary-active}"
    textColor: "{colors.primary-foreground}"
  button-secondary:
    backgroundColor: "{colors.background}"
    textColor: "{colors.foreground}"
    typography: "{typography.label-md}"
    rounded: "{rounded.sm}"
    padding: 0 12px
    height: 32px
  button-ghost-hover:
    backgroundColor: "{colors.muted}"
    textColor: "{colors.foreground}"
  button-danger:
    backgroundColor: "{colors.danger}"
    textColor: "{colors.primary-foreground}"
    typography: "{typography.label-md}"
    rounded: "{rounded.sm}"
    padding: 0 12px
    height: 32px
  button-small:
    typography: "{typography.label-sm}"
    rounded: "{rounded.sm}"
    padding: 0 8px
    height: 24px
  input:
    backgroundColor: "{colors.muted}"
    textColor: "{colors.foreground}"
    typography: "{typography.body-sm}"
    rounded: "{rounded.sm}"
    padding: 0 8px
    height: 32px
  chip:
    backgroundColor: "{colors.muted}"
    textColor: "{colors.muted-foreground}"
    typography: "{typography.caption-sm}"
    rounded: "{rounded.full}"
    padding: 2px 8px
  chip-accent:
    backgroundColor: "{colors.accent-soft}"
    textColor: "{colors.accent-soft-foreground}"
    typography: "{typography.caption-sm}"
    rounded: "{rounded.full}"
    padding: 2px 8px
  count:
    textColor: "{colors.muted-foreground}"
    typography: "{typography.caption-sm}"
  section-header:
    textColor: "{colors.muted-foreground}"
    typography: "{typography.caption-sm}"
    padding: 8px 8px 4px
  sidebar-row:
    backgroundColor: "{colors.sidebar}"
    textColor: "{colors.sidebar-foreground}"
    typography: "{typography.body-md}"
    rounded: "{rounded.md}"
    padding: 0 8px
    height: 32px
  sidebar-row-selected:
    backgroundColor: "{colors.sidebar-accent}"
    textColor: "{colors.foreground}"
  list-row:
    backgroundColor: "{colors.background}"
    textColor: "{colors.foreground}"
    typography: "{typography.title-sm}"
    padding: 8px 12px
    height: 56px
  list-row-hover:
    backgroundColor: "{colors.muted}"
    textColor: "{colors.foreground}"
  list-row-subtitle:
    textColor: "{colors.muted-foreground}"
    typography: "{typography.caption}"
  card:
    backgroundColor: "{colors.background}"
    textColor: "{colors.foreground}"
    rounded: "{rounded.lg}"
  popover:
    backgroundColor: "{colors.popover}"
    textColor: "{colors.foreground}"
    typography: "{typography.body-sm}"
    rounded: "{rounded.xl}"
    padding: 8px
  dialog:
    backgroundColor: "{colors.popover}"
    textColor: "{colors.foreground}"
    typography: "{typography.body-sm}"
    rounded: "{rounded.xl}"
    padding: 16px
  dialog-title:
    textColor: "{colors.foreground}"
    typography: "{typography.title-md}"
  notice-warning:
    textColor: "{colors.warning-text}"
    typography: "{typography.caption}"
  notice-danger:
    textColor: "{colors.danger-text}"
    typography: "{typography.caption}"
  notice-success:
    textColor: "{colors.success-text}"
    typography: "{typography.caption}"
  notice-info:
    textColor: "{colors.info-text}"
    typography: "{typography.caption}"
  status-dot-success:
    backgroundColor: "{colors.success}"
    size: 8px
    rounded: "{rounded.full}"
  status-dot-warning:
    backgroundColor: "{colors.warning}"
    size: 8px
    rounded: "{rounded.full}"
  status-dot-info:
    backgroundColor: "{colors.info}"
    size: 8px
    rounded: "{rounded.full}"
  focus-ring:
    backgroundColor: "{colors.ring}"
  divider:
    backgroundColor: "{colors.hairline}"
  border:
    backgroundColor: "{colors.border}"
  code-block:
    backgroundColor: "{colors.muted}"
    textColor: "{colors.foreground}"
    typography: "{typography.mono-sm}"
    rounded: "{rounded.md}"
    padding: 8px 12px
  title-large:
    textColor: "{colors.foreground}"
    typography: "{typography.title-lg}"
  body-text:
    textColor: "{colors.foreground}"
    typography: "{typography.body-md}"
  avatar-overflow:
    backgroundColor: "{colors.muted}"
    textColor: "{colors.foreground}"
    typography: "{typography.micro}"
    rounded: "{rounded.full}"
---

# SkillBinder design

## Overview

SkillBinder is a native macOS-first desktop app for developers who use several coding agents. It keeps a library of skills and copies the active ones into each agent's skill folders and into workspaces. The UI should feel like a calm, trustworthy system utility: quiet surfaces, clear hierarchy, color only where it carries meaning.

The direction is **Inset canvas** (chosen 2026-11-18). Depth comes from layered surfaces rather than lines: the window and sidebar share a tinted canvas, and each page sits on a raised white panel inset from the window edges. One iris accent marks the primary action, focus, and the current selection. Everything else stays neutral.

The design is built from GPUI Kit components, themed through Kit's theme and styled through their public API. The tokens above are the source of truth; the design-system crate holds their generated code form (see Implementation).

## Colors

Light values are the plain names; each color that changes in dark mode has a `-dark` sibling. A color with no `-dark` sibling keeps its value in both modes.

- **sidebar**: the window canvas, behind the sidebar, the header, and the inset panel. Never used as a card or control fill.
- **background**: the inset page panel, cards, and list rows. In dark mode it is a step lighter than the canvas, so the panel still reads as raised.
- **foreground / muted-foreground**: primary text and secondary text (descriptions, counts, section headers, placeholders). `muted-foreground` passes 4.5:1 on `background`, `muted`, and `sidebar`.
- **sidebar-foreground**: unselected sidebar labels. **sidebar-accent**: the selected sidebar row, drawn as a raised white pill (dark: a lifted gray).
- **muted**: neutral fills: hover, input fields, segmented tracks, code blocks. It is the only hover fill; do not mix in foreground-at-alpha hovers.
- **border**: control and card outlines. **hairline**: dividers between rows and inside panels.
- **popover**: menus, popovers, dialogs, tooltips.
- **primary** (iris): the primary button, links, the focus ring (`ring`), selected checkboxes and switches. One primary action per view.
- **accent-soft / accent-soft-foreground**: the tinted chip for a highlighted state, such as Updates or a chosen filter.
- **success, warning, danger, info**: status fills (dots, button fills, progress). Their `-text` siblings are the readable text and icon colors for notices on `background`. `info` is the update cyan of the approved prototypes. `danger` is the destructive button fill in both modes.
- **preset-***: the categorical palette a user picks a preset's color from (gray, red, orange, yellow, green, blue, purple, pink), toned to keep contrast on both sidebars. It colors a preset's dot and glyph and nothing else. A custom preset color is the same in both modes.

## Typography

The system font (SF Pro through `.SystemUIFont`) everywhere, SF Mono for paths and commands. Every level uses a line box 1.4 times its size, tighter than GPUI's default, except `title-lg` and `micro`. The medium weight is 520, not 500: GPUI resolves 500 to SF Pro Regular.

- **title-lg** 20/600: onboarding step titles and page titles.
- **title-md** 16/600: dialog titles.
- **title-sm** 14/600: list row titles (skill slugs), card titles.
- **body-md** 14/400: sidebar labels and body copy.
- **body-sm** 13/400: menu items, dialog body, inputs.
- **label-md** 13/520 and **label-sm** 12/520: button labels at Kit's medium and small sizes, chips.
- **caption** 12/400: row descriptions, notices, details.
- **caption-sm** 11/520: section headers, counts, folder tags. Counts use tabular figures.
- **micro** 10/600: the `+N` overflow in avatar stacks only.
- **mono-sm** 12/400: paths and commands.

## Layout

A 4 px rhythm with a 6 px half step for tight controls: `2xs` 2, `xs` 4, `sm` 6, `md` 8, `lg` 12, `xl` 16, `2xl` 24, `3xl` 32. Control heights follow Kit: 24 (small) and 32 (medium). The window header is 44; sidebar rows are 32; library list rows are at least 56.

- The window is a two-pane layout: the sidebar on the canvas, the page in an inset panel 6 px from the window's top-right-bottom edges. The panel's content uses a 16 px gutter.
- Icon sizes come from one scale: 12, 14, 16, 20. An inline icon beside caption text is 14; a sidebar or row icon is 16.
- Dialogs are sized from the window (at most 92% wide and 90% tall) or from a fixed width of 320 or 420.

## Elevation & Depth

Four levels. Raised and overlay surfaces get a hairline ring plus a soft drop shadow, never a heavy border. GPUI draws a blur about twice as wide as CSS, so blur values here are half the visual target.

- **Flat**: rows, cards inside the panel. A card has a 1 px `border` ring and no drop shadow.
- **Raised**: the inset page panel, the selected sidebar row, the selected segment, the dragged row. Ring `rgba(0,0,0,0.08)` at 0.5 px, shadow 0 1 2 `rgba(0,0,0,0.08)`; the panel adds 0 6 16 -6 `rgba(0,0,0,0.08)`.
- **Overlay**: menus, popovers, the progress pill. Ring `foreground` at 10%, shadows 0 10 15 -3 and 0 4 6 -4 at 10% black (the current `menu::shadow`).
- **Modal**: dialogs and toasts, Kit's large shadow over the dimmed window. GPUI has no backdrop blur; the blocking wash is the page color at 70%.

In dark mode shadows fade, so surfaces separate by lightness: canvas `#111113`, panel `#1B1B1F`, popover `#222227`.

## Shapes

Radii scale with the object's size: `xs` 4 for tags and image tiles, `sm` 6 for controls (buttons, inputs, menu rows), `md` 8 for sidebar rows and selections, `lg` 10 for cards, the inset panel, and grid tiles, `xl` 12 for menus, popovers, and dialogs, `full` for chips, avatars, dots, and pills.

## Components

- **Buttons**: Kit `Button`. Primary is iris, one per view; secondary is outlined; ghost for toolbar and row actions; danger only for an action that removes something. Labels use `label-md` (small buttons `label-sm`). Disabled buttons lose the hand cursor.
- **Inputs and search**: Kit `Input` on a `muted` fill with a 14 px search glyph.
- **Chips and tags**: Kit `Tag`, fully rounded; neutral by default, `accent-soft` when it marks a highlighted state.
- **Lists**: a card holding rows separated by inset `hairline` dividers; a row has an optional leading icon or logo, a title, a one-line description, and trailing marks. Hover fills `muted`; an inactive row is drawn at 50% opacity.
- **Sidebar**: rows on the canvas; the selected row is a raised `sidebar-accent` pill. Section headers in `caption-sm`.
- **Menus and popovers**: `popover` surface at overlay elevation, 8 px padding, rows 32 high with `sm` radius.
- **Dialogs**: Kit `Dialog` with a `title-md` title, `body-sm` body, and a right-aligned footer: secondary then primary, gap 8.
- **Notices**: an icon and caption text in the tone's `-text` color, on no fill; a blocking problem uses Kit `Alert`.
- **Empty states**: Kit `Empty`: glyph, one line, optional action.
- **Toasts**: Kit `Notification`, bottom center.
- **Tooltips**: Kit `Tooltip`, wrapping past 240 px.
- **Checkboxes, switches, avatars, spinners, progress**: Kit's own, themed through the tokens.

## Do's and Don'ts

- Do take every size, color, radius, and shadow from these tokens; a raw `px(13.)` or hex literal in UI code is a bug.
- Do reach for a GPUI Kit component before building one; the app keeps no look-alike of a Kit control.
- Do keep color meaningful: iris for primary, focus, and selection; status colors for status. Never color for decoration.
- Don't add borders to separate the sidebar from the page; the raised panel does that.
- Don't use more than one primary button in a view.
- Don't use `FontWeight::MEDIUM`; use the 520 medium weight.
- Don't dim text with ad hoc alphas; use `muted-foreground`, or 50% opacity for an inactive row.

## Implementation

This file is the source of truth for every token. `cargo run -p xtask -- design-tokens` generates their code form in the design-system crate, and every color lands in exactly one of two places:

- **The GPUI Kit theme file** (light and dark, loaded at startup) takes the colors Kit has a key for: `background`, `foreground`, `sidebar`, `sidebar-foreground`, `sidebar-accent`, `muted`, `muted-foreground`, `border`, `popover`, `ring`, `primary` with its hover, active, and foreground, and the `success`, `warning`, `danger`, and `info` fills. It also takes `radius` (`rounded.sm`) and `radius.lg` (`rounded.lg`). Kit's `success.foreground`, `warning.foreground`, `danger.foreground`, and `info.foreground` keep Kit's meaning, the text on the filled status color: the generator sets each to whichever of `primary-foreground` and `foreground` contrasts more with that fill. A `-text` tone never goes there.
- **Generated Rust constants** take everything Kit has no key for, as light and dark pairs: `hairline`, `accent-soft`, `accent-soft-foreground`, the `-text` tones, and `preset-*`. The same file holds spacing, type levels, radii, icon sizes, opacity, and elevation.

The generator fails on a color with no destination rather than skipping it. `cargo run -p xtask -- verify` fails while the generated files disagree with this file, so a token change is an edit here followed by the generator, in one commit. Lint this file with `npx -y @google/design.md@0.4.0 lint DESIGN.md`; the only expected warnings are orphaned `-dark` and `preset-*` colors. The build plan is in [docs/design-system/plan.md](docs/design-system/plan.md).
