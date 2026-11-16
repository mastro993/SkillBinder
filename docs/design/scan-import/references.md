# References: skill import redesign

Mobbin covers web and iOS only. SkillBinder is a native macOS desktop app, so every reference below is a **web pattern used as a stand-in**. `image_url` thumbnails expire in about 30 days; `mobbin_url` is the lasting link.

## 1. Import dialog

### Selection list, count in the button
- **Cosmos**, "We found 2 boards" — https://mobbin.com/screens/fc750b0c-7575-47e2-a438-fd352fbc6ee7 — image: https://mobbin.com/api/mcp/short/mqpkFp7q
  Lends: the closest match to "found on device". Headline states the count found, one line explains what each item becomes and what is not supported, every row arrives pre-checked (trailing check), a "Deselect all" text link sits right of the list header, and the primary button repeats the live count ("Import 2 boards").
- **Workable**, Import from templates — https://mobbin.com/screens/50f88d90-14e8-4701-b2d4-e1a782371cb6 — image: https://mobbin.com/api/mcp/short/KlKrMpD4
  Lends: a minimal modal with a plain checkbox list and a footer primary button whose label tracks the selection ("Import 2 templates").
- **Devin**, Add repositories — https://mobbin.com/screens/217223cc-d5b9-46fa-ab5e-8de35e31f974 — image: https://mobbin.com/api/mcp/short/jwlnCUrU
  Lends: a two-line row (name plus a muted owner/source line) that fits slug plus source path, a tri-state select-all checkbox inside the search row, and Cancel next to a counted primary ("Add 1 repository").
- **Vimeo**, Select videos — https://mobbin.com/screens/73ba2aac-aefe-49a3-b15d-de316c8db5af — image: https://mobbin.com/api/mcp/short/VVIY6I7s
  Lends: a "1 selected" counter beside the master checkbox above the list, an outlined selected row, and row metadata columns (visibility, date) on the right.
- **Qatalog**, source picker — https://mobbin.com/screens/35f3c81c-0e42-4965-9676-cabfe75b21bd — image: https://mobbin.com/api/mcp/short/LAZ7odRB
  Lends: the selection count sits bottom-left in the footer ("1 item selected"), opposite the primary ("Use selected"), with a source/date/size line under each name.

### Expandable alternatives (versions)
- **Shopify**, Add products — https://mobbin.com/screens/e51e53fc-48f4-4098-9854-17be91bb2a91 — image: https://mobbin.com/api/mcp/short/EbbElr0Y
  Lends: a parent row with indented child variant rows ("50g - Medium", "100g - Large"), each with its own checkbox, and a footer chip counting variants selected ("4 variants selected"). Maps directly to a skill row with its found versions.
- **Square**, Add items — https://mobbin.com/screens/321c446f-30d6-4275-bd0e-f404cc87d862 — image: https://mobbin.com/api/mcp/short/hJco30qU
  Lends: a collapsed state that only hints at alternatives with a muted subline ("2 variations"), so the list stays one row per item until the user opens one.
- **StackAI**, Import from Websites — https://mobbin.com/screens/c9dd80b9-d2c0-4058-ba96-a4e3a7a900d6 — image: https://mobbin.com/api/mcp/short/mgpM3X1g
  Lends: a chevron disclosure before the checkbox on expandable rows, leaf rows without a chevron, all in one table with a header select-all checkbox.

### Per-row warnings and blocked rows
- **QuickBooks**, Review and confirm the leads — https://mobbin.com/screens/5a8e242e-8f7e-4ea6-b16e-bcd44a6b69cc — image: https://mobbin.com/api/mcp/short/0QUQZPOz
  Lends: a dismissible warning banner above the list ("Some leads are missing some information and can't be imported"), valid rows pre-checked, blocked rows left unchecked below them, and an orange icon in the row that explains why.
- **Remote**, Review time off data — https://mobbin.com/screens/d04c9f64-c644-4b9c-8e43-93de5b2fdbc0 — image: https://mobbin.com/api/mcp/short/GMHXcvjE
  Lends: a leading status icon per row (green check or red warning with count), the reason in a tooltip on the flagged cell, and a primary that counts only importable rows ("Continue with 1 time off record").
- **Melio**, Review vendor details — https://mobbin.com/screens/997a8e01-8187-42f8-9c77-cc7fc6fb5303 — image: https://mobbin.com/api/mcp/short/FUlJQodc
  Lends: an info banner with the ready/blocked split ("2 of 10 vendors are ready to import"), a small red dot on blocked rows plus inline red reason text, and a "Show only … with errors" toggle; the primary counts only ready rows ("Import 2 vendors").
- **7shifts**, Import employees — https://mobbin.com/screens/6b45f042-7f89-46f1-b46f-8270604842b8 — image: https://mobbin.com/api/mcp/short/EyTDXjrg
  Lends: a fraction on the button ("Import employees (1/2)") that tells the user some found items will be skipped, with a yellow caution banner above.

### Scanning (loading) state
- **HoneyBook**, Import contacts — https://mobbin.com/screens/b79ea0dd-00ab-42c2-bbd8-266c4e9a8143 — image: https://mobbin.com/api/mcp/short/kQZGFdG8
  Lends: the same modal frame and title as the result view with only a centred spinner and "Scanning your file…", so the dialog does not resize or jump when results arrive.
- **Base44**, App Store Scan In Progress — https://mobbin.com/screens/4a18bfd3-4480-4beb-8b5f-8227ee274532 — image: https://mobbin.com/api/mcp/short/UKGWcAex
  Lends: a spinner with a primary line ("Loading app source code…") and a muted secondary line saying what is being checked; useful for "Looking in Claude, Codex, …".
- **Dovetail**, Topics — https://mobbin.com/screens/b5dc29eb-d024-4216-b5ea-04b37e308e2f — image: https://mobbin.com/api/mcp/short/Rx1fjrCT
  Lends: the lightest version, a single muted line ("Scanning through your data…") under the dialog title with no spinner, for scans that finish fast.

### Empty "nothing new found" state
- **TikTok**, Select videos — https://mobbin.com/screens/13b261e2-2fe2-44ed-93a4-da9bc93b4612 — image: https://mobbin.com/api/mcp/short/0JNkGrZm
  Lends: the exact case. A picker modal finds nothing new: icon, "No favorite videos to add", the reason ("All your favorite videos are already in collections"), and a single full-width "Done".
- **7shifts**, Match your restaurant roles — https://mobbin.com/screens/e46dd2ef-b8d7-481a-9e3c-eb421ee4f667 — image: https://mobbin.com/api/mcp/short/FMuIvxEY
  Lends: a positive framing inside a bordered list area ("All roles are matched! There are no roles to match"), keeping the footer so the flow can continue.
- **Langdock**, Inbox — https://mobbin.com/screens/bdb049f8-bb88-499a-bf90-abf703a9ecc8 — image: https://mobbin.com/api/mcp/short/oalHQGkn
  Lends: a quiet tray icon, bold one-line title, and one muted sentence saying when items will appear; good restraint for a utility app.

## 2. Library entry

### Empty library with one primary action
- **ManyChat**, Contacts — https://mobbin.com/screens/69ca4148-4ce7-49eb-99ea-fad4ee80942e — image: https://mobbin.com/api/mcp/short/9BJNMWIA
  Lends: a centred empty card ("You don't have any contacts yet"), one sentence of what import does, a single "Import" primary, while the header keeps the same Import action top-right so it does not vanish once the list fills.
- **Customer.io**, Imports — https://mobbin.com/screens/ae546953-a506-491d-a969-4297e1df96ec — image: https://mobbin.com/api/mcp/short/OFjXmRC3
  Lends: the empty state sits inside the table's own frame (column headers stay visible), with a small icon, a title, a short explanation, one primary ("Import a CSV") and a muted learn-more link.
- **Attio**, Import history — https://mobbin.com/screens/e0a7196a-6eb9-4f1c-887f-86e949331617 — image: https://mobbin.com/api/mcp/short/ONU1uOT6
  Lends: compact version: illustration, "No imports", one line, one icon-led primary ("Start new import").
- **Plane**, CSV Importer — https://mobbin.com/screens/81706f26-e69f-4831-8e7f-dee029e33ea3 — image: https://mobbin.com/api/mcp/short/JvYEWt04
  Lends: no button in the empty body at all; the empty state only explains, and the single "Import" lives in the page header. Useful if the header button is the one entry point.

### Header button with count badge
Weak coverage on Mobbin for a non-notification button with a badge; these are the nearest stand-ins.
- **HoneyBook**, dashboard header — https://mobbin.com/screens/1afda499-c538-4a00-8900-d0a43dffcdfe — image: https://mobbin.com/api/mcp/short/dt9aA67s
  Lends: small blue numeric badges pinned to the top-right corner of header icon buttons (bell "10", sparkle "1"); the badge colour matches the brand accent rather than alarm red.
- **Deel**, Notifications — https://mobbin.com/screens/594a2e48-8cc1-4466-a8eb-8f92bca12873 — image: https://mobbin.com/api/mcp/short/BrGTP2Dd
  Lends: a capped badge ("9+") on a round header icon button, plus the count repeated as a dark pill next to the section title once opened ("Activity Feed 20").
- **Plane**, Initiatives — https://mobbin.com/screens/c91a7ea7-d970-409b-861a-77cee3ccc286 — image: https://mobbin.com/api/mcp/short/mZq6Pnvl
  Lends: a plain muted count beside a header label ("All Initiatives 4") and a red dot badge on a small header icon; shows how quiet a count can be.
- **Magnific**, sidebar — https://mobbin.com/screens/98720aa7-e012-4cdc-a841-15e65a0b3599 — image: https://mobbin.com/api/mcp/short/pxWKSgio
  Lends: a tiny coloured badge on a compact icon in a dense toolbar, the panel it opens has an empty state ("Inbox is empty") for when the count reaches zero.

## 3. Onboarding import step

- **Cosmos**, "We found 2 boards" — https://mobbin.com/screens/fc750b0c-7575-47e2-a438-fd352fbc6ee7 — image: https://mobbin.com/api/mcp/short/mqpkFp7q
  Lends: a full-page onboarding step (not a modal) that lists found items inline, all pre-checked, with "Deselect all" and the counted primary placed above the list so it is visible without scrolling. Back chevron top-left acts as the exit.
- **Loops**, Import contacts — https://mobbin.com/screens/4b07d82f-2630-45ae-afe1-285b2dc9497d — image: https://mobbin.com/api/mcp/short/bc6AQyNT
  Lends: a step-progress dash row under the title, a single card with one primary ("Let's do it"), and a muted text-link Skip below ("Skip and do this later") that is clearly secondary.
- **Literal**, Connecting to Goodreads (flow) — https://mobbin.com/flows/ffef5c13-54db-408f-be5a-c959185ec6b5?tab=screens&utm_source=claude-code&utm_medium=ai-assistant&utm_source_platform=mcp — image: https://mobbin.com/api/mcp/short/1t0WOGcy
  Lends: a "2/3" progress ring in the step header and two equal-weight choices ("Import from Goodreads" vs "Start new library"), framing Skip as a real path rather than an escape.
- **Substack**, Import your mailing list — https://mobbin.com/screens/40375133-7443-407c-ab4a-bbf5921d16fa — image: https://mobbin.com/api/mcp/short/G0ilPGNr
  Lends: a reassuring line above Skip ("Don't have an email list yet? Substack will help you build one.") and a full-width secondary Skip button under the import card.
- **Plane**, What brings you to Plane? — https://mobbin.com/screens/4de2853a-2775-4a3a-ba68-129a3627cde1 — image: https://mobbin.com/api/mcp/short/xxry398v
  Lends: a narrow centred column of checkbox rows where checked rows get an accent border and tint, full-width primary "Continue" with a plain "Skip" text link directly under it. Good model for the inline list's row styling.
- **Cloaked**, Importing accounts (flow) — https://mobbin.com/flows/00b0b290-fc75-476c-b91e-a58f442dc9e1?tab=screens&utm_source=claude-code&utm_medium=ai-assistant&utm_source_platform=mcp — image: https://mobbin.com/api/mcp/short/O1wqKdbl
  Lends: the end of the flow lands on the filled library with a toast ("3 items successfully imported"), confirming the result in place instead of a separate success screen.
