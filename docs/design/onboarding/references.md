# References — SkillBinder onboarding and follow-up surfaces

Source: Mobbin (web screens). Mobbin has no macOS desktop coverage; every reference below is a **web stand-in** for a native macOS window. `image_url` thumbnails expire in about 30 days; `mobbin_url` is the lasting link.

Mobbin notice (verbatim):

> ---
>
> **Your Mobbin AI usage is high this month**
>
> Usage is unlimited during beta. If your usage stays at this level, you may need to purchase more credits once limits take effect. [View usage and plan options](https://mobbin.com/settings/billing-and-usage?utm_source=claude-code&utm_medium=ai-assistant&utm_source_platform=mcp)

## frame — 2–3 step setup wizard

- **Figma — Dev Mode welcome dialog (step 1 of 4)**
  - mobbin_url: https://mobbin.com/screens/68c2e030-6eac-46f0-9b0d-9aead62e27c1
  - image_url: https://mobbin.com/api/mcp/short/zJMbzlyO
  - Lends: split layout in one window: dark left panel with title + numbered step list (active step highlighted), right pane holds the step; footer Back + "Next: Theme" bottom-right that names the next step.
- **Klaviyo — Set up reviews, "Install widgets" step**
  - mobbin_url: https://mobbin.com/screens/399707d8-fc2f-47db-9a10-29433f87fbf6
  - image_url: https://mobbin.com/api/mcp/short/thveDIPy
  - Lends: centered single column with a tiny segmented progress bar + step name in the header; Back / Skip step / Next grouped top-right; "Recommended" badge on the default option.
- **Clerk — Claim your application (vertical stepper)**
  - mobbin_url: https://mobbin.com/screens/c0ce2e61-3247-40e2-bd28-2e09cefe7733
  - image_url: https://mobbin.com/api/mcp/short/QNqG1fO3
  - Lends: calm centered column with an inline vertical stepper; only the current step is expanded, upcoming steps are faded titles.
- **OpenAI Platform — ChatGPT app setup (MCP Server step)**
  - mobbin_url: https://mobbin.com/screens/fbb5e111-6a6c-43cf-801a-a623ae4b05c0
  - image_url: https://mobbin.com/api/mcp/short/vPRU6bkt
  - Lends: horizontal labelled stepper on top; Back bottom-left, Continue bottom-right at content width (classic dev-tool wizard footer).
- **Melio — Complete details (left step list)**
  - mobbin_url: https://mobbin.com/screens/03ef6cc6-505c-4277-bfe2-b0c068a141c1
  - image_url: https://mobbin.com/api/mcp/short/ftr6F6Pn
  - Lends: left checkmarked step list beside content, full-width footer bar with Cancel left, Back + Continue right; disabled Continue until valid.
- **Mixpanel — Set Up Guide (Step 1 Start Here)**
  - mobbin_url: https://mobbin.com/screens/cd5e2054-f3f7-4101-a801-9b4f6dc6046c
  - image_url: https://mobbin.com/api/mcp/short/JVQDn7QV
  - Lends: modal with left pill step list (Get Started / Connect Data / Verify Connection) and "STEP 1" eyebrow above the step title.

## git — install a required/optional dependency

- **Sentry — Configure Next.js SDK (onboarding step 2)**
  - mobbin_url: https://mobbin.com/screens/68401530-20c8-4f67-9ff3-e29136989fb2
  - image_url: https://mobbin.com/api/mcp/short/XkDuMWiC
  - Lends: "Automatic (Recommended)" one-command path with a manual alternative collapsed below; footer shows a live "Waiting for…" status between Skip and the primary action.
- **Clerk — "Watching for users" overview**
  - mobbin_url: https://mobbin.com/screens/0441b8ff-f6b1-4b01-8601-4b53a8153c5a
  - image_url: https://mobbin.com/api/mcp/short/wdQFyGbx
  - Lends: passive detection: a small spinner label ("Watching for users") + copy "The moment we detect…, installation is complete" — no manual re-check needed; terminal box with copy button.
- **Vercel — Install Coding Agent Plugin card**
  - mobbin_url: https://mobbin.com/screens/24cd6b8c-c46b-4fe9-a492-ec559f00bd48 (copy state: https://mobbin.com/screens/d6205dc6-60f5-429f-824d-906c1f5751c5)
  - image_url: https://mobbin.com/api/mcp/short/FlATUT63 (copied: https://mobbin.com/api/mcp/short/ozdIbGw9)
  - Lends: compact bordered card with one-line explanation and a `$ command` row whose copy icon flips to a check after copying; dismissible.
- **Graphite — Get started checklist (install and authenticate the CLI)**
  - mobbin_url: https://mobbin.com/screens/ad55bdb3-3332-4b54-9f37-7537263c999c
  - image_url: https://mobbin.com/api/mcp/short/W8knzrw8
  - Lends: completed steps collapse to a struck-through checked row; "x of y tasks complete" + "Complete setup later" escape hatch.
- **Cursor — Connecting to GitHub (waiting card)**
  - mobbin_url: https://mobbin.com/screens/893ffaa1-74b0-48d5-8f16-a40a616b80d6
  - image_url: https://mobbin.com/api/mcp/short/6H6SgBdt
  - Lends: minimal waiting state while an external step finishes: small spinner + status word, eyebrow (GitHub), title, one line of what is happening.
- **Mistral AI — Build your first workflow (numbered command boxes)**
  - mobbin_url: https://mobbin.com/screens/fd7807e2-5bb5-479e-ba9f-f4994f63ca0c
  - image_url: https://mobbin.com/api/mcp/short/ng3gYZzY
  - Lends: numbered read-only command fields with copy buttons, plus a "copied to clipboard" confirmation toast.

## agents — choose which integrations/apps to manage

- **Dropbox Dash — Connect your go-to apps**
  - mobbin_url: https://mobbin.com/screens/86171fe5-5480-4ef0-b29c-fe73f32d5aac
  - image_url: https://mobbin.com/api/mcp/short/Bo9Qt5pI
  - Lends: grouped row standing for several apps (logo cluster + "Gmail, Google Drive, and Calendar" subtitle) with check/plus state; Back arrow left, Continue right in footer.
- **HubSpot — What tools do you use? (2 of 4)**
  - mobbin_url: https://mobbin.com/screens/fbf1de70-bfdc-4d83-aa83-08d90e725904
  - image_url: https://mobbin.com/api/mcp/short/LxqJpYQW
  - Lends: checkbox rows with logo + name, several pre-selected (tinted background); "2 of 4" counter; left step list; "Skip, for now" + Next.
- **Fabric — What apps do you currently pay for?**
  - mobbin_url: https://mobbin.com/screens/902c0a2a-0a4d-407c-869c-83cd82ea32c6
  - image_url: https://mobbin.com/api/mcp/short/ZSMXN7T0
  - Lends: calm centered single column of bordered rows: logo, name, trailing checkbox — closest to a shadcn list.
- **Gumloop — Connect apps to supercharge your agent**
  - mobbin_url: https://mobbin.com/screens/b6c20b5e-47af-413c-81e9-5fadf31bde3a
  - image_url: https://mobbin.com/api/mcp/short/f3lO6Pv0
  - Lends: rows split into "Connected" (with subtitle "Included in Gumloop") and "All connectors" sections; dot pager + Back / Continue.
- **ClickUp — Select source of import**
  - mobbin_url: https://mobbin.com/screens/dbe9f797-d667-4cf4-b7ef-600c29c38e79
  - image_url: https://mobbin.com/api/mcp/short/eIXDQWJf
  - Lends: dense single-line rows with logo + name and right-aligned small meta tags (where a path could sit).
- **Cal.com — No calendar apps (empty state)**
  - mobbin_url: https://mobbin.com/screens/b0cd09a4-5358-43bd-9aa4-9694d7a2aaec
  - image_url: https://mobbin.com/api/mcp/short/xkhs0eym
  - Lends: "none found" empty state inside the list's card: icon, title, one line, one outline button.
- **PandaDoc — No authorized apps yet (empty state)**
  - mobbin_url: https://mobbin.com/screens/f6680526-0150-4cce-b8c4-4211797a6a80
  - image_url: https://mobbin.com/api/mcp/short/b2pY5jGp
  - Lends: text-only empty state ("Once you … we'll show it here") with a single secondary button — no illustration.

## settings-agents — integrations with switches + dependency status row

- **Linear — Preferences › Coding tools**
  - mobbin_url: https://mobbin.com/screens/9ac66835-aa7f-469f-ace3-a915c2cf2dfd
  - image_url: https://mobbin.com/api/mcp/short/zU9roHfL
  - Lends: almost exactly our list: Claude Code, Codex, Cursor… rows with icon, name, one-line note ("Opens in your terminal. Requires the Linear desktop app. Learn more") and a trailing switch.
- **Qatalog — Enabled Connectors**
  - mobbin_url: https://mobbin.com/screens/bf24e755-2a66-4aa1-b64b-ad156bbd0641
  - image_url: https://mobbin.com/api/mcp/short/fowNuBes
  - Lends: enabled rows tinted with "Enabled" label + switch; disabled rows muted; an inline info note row separating groups.
- **Unify — Settings › Integrations**
  - mobbin_url: https://mobbin.com/screens/6717566f-1d46-4fb0-8432-8f7554342584
  - image_url: https://mobbin.com/api/mcp/short/dGuXAR3l
  - Lends: row status variants: "● Connected" text vs Connect button vs disabled button, with a small inline note under a row explaining why it's unavailable — model for the Git status row + Install.
- **Supabase — Settings › Add-ons**
  - mobbin_url: https://mobbin.com/screens/dedc157f-70bc-4076-ad0d-ae8306af8e31
  - image_url: https://mobbin.com/api/mcp/short/2b2Uigam
  - Lends: compact status badge ("DISABLED") right-aligned on a row with title, description and a "learn more" link.
- **Mintlify — Settings › Integrations**
  - mobbin_url: https://mobbin.com/screens/e700ca68-4482-467d-abe8-c66cbbd663fa
  - image_url: https://mobbin.com/api/mcp/short/hu7UN9Vs
  - Lends: "Enabled" section vs "Available" section with "+ Connect" buttons; toast confirms a toggle-off.

## toast — persistent "Found X. Connect it?" notice

- **beehiiv — "Welcome to beehiiv" corner card**
  - mobbin_url: https://mobbin.com/screens/67b4162b-dbc1-440b-83bc-01ca0e98f145
  - image_url: https://mobbin.com/api/mcp/short/r9nJJ7Yd
  - Lends: bottom-right card that waits for an answer: title, one line, "Remind Me Later" (ghost) + "Let's go" (primary), close X.
- **HoneyBook — template saved toast with action**
  - mobbin_url: https://mobbin.com/screens/dd7106fa-f9cd-4c3b-bd71-57d1699b113f
  - image_url: https://mobbin.com/api/mcp/short/yOmx8ril
  - Lends: top-right card toast with accent stripe, bold title, two-line body, right-aligned text action ("View and customize template") and close.
- **Quicken — snackbar with Undo + coach-mark with two actions**
  - mobbin_url: https://mobbin.com/screens/e9042731-b020-4b97-b8b7-b5ad5a97db06
  - image_url: https://mobbin.com/api/mcp/short/IAEBXCDk
  - Lends: compact dark snackbar with an inline action and X; the coach-mark shows the "Don't show again" + "OK" pair for a decline that sticks.
- **Unify — "Slack integration enabled" toast**
  - mobbin_url: https://mobbin.com/screens/6717566f-1d46-4fb0-8432-8f7554342584
  - image_url: https://mobbin.com/api/mcp/short/dGuXAR3l
  - Lends: bottom-right toast with status icon, title + one-line body, close X — size/position baseline for a calm shadcn-like Sonner toast.

Gap: Mobbin had no direct "detected X — connect it?" toast with primary + dismiss buttons in a dev tool; beehiiv is the closest structural match. macOS-native notices (e.g. Raycast, Docker Desktop) are not on Mobbin.
