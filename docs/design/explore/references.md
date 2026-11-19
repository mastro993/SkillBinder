# References: Explore

Source: Mobbin MCP, searched 2026-11-19. Mobbin covers web and iOS only; every reference below is a **web** screen used as a stand-in for the native desktop page. `mobbin_url` is the lasting link; `image_url` is a short-lived thumbnail (expires in about 30 days).

## A. Catalog home: popular section + search + one-click install

| App | mobbin_url | image_url | What it lends |
|---|---|---|---|
| Cal.com (App Store) | https://mobbin.com/screens/eda2c950-18ba-426d-9428-a98b991e8f68 | https://mobbin.com/api/mcp/short/aaPlqUXU | Sidebar-nested "App Store" / "Installed apps" pair; page stacks "Most Popular" then "Recently added" horizontal card rails under one search field. |
| Langdock (Integrations) | https://mobbin.com/screens/1220a80f-e373-46cb-91b8-c03e048f275b | https://mobbin.com/api/mcp/short/6n9kLnET | "Popular in workspace" grid above "All integrations", per-card trailing Connect button, inline "Connected" state on an already-added card, status toast after the action. |
| Airtable (Marketplace Extensions) | https://mobbin.com/screens/403b7318-4f7e-410a-9607-305c36e4968f | https://mobbin.com/api/mcp/short/nuDJirhA | Compact two-column rows (icon, name, one-line description, small "Add" button), with tabs that split featured / by-publisher / all, and search in the tab bar. |
| Adobe Express (Add-ons) | https://mobbin.com/screens/b6b4ebcf-54f8-43d0-a9e0-84b111c0a8d7 | https://mobbin.com/api/mcp/short/L6No7AFJ | Dense three-column list rows grouped under section headings, each with a pill "Add" button. Closest to a desktop-density catalog. |
| v0 (Extensions: Marketplace) | https://mobbin.com/screens/bcc629fe-8ac0-4b61-919f-4ddf02953ea4 | https://mobbin.com/api/mcp/short/HOH8Y2rk | VS Code-style narrow result list (icon, name, publisher, install count, stars, inline Install) beside a detail pane with Install, repository link, version, and categories. |
| Vercel (Marketplace) | https://mobbin.com/screens/82426445-002e-4bb6-ab23-72e90b793690 | https://mobbin.com/api/mcp/short/7EVWclpB | Full-width search at the top, a "Featured" curated row, a left facet column (type/category), and an "Installed Integrations" header button. |
| Cursor (Plugins) | https://mobbin.com/screens/e2e02fae-0e4a-4eaf-8001-4d951711862a | https://mobbin.com/api/mcp/short/eIywKDAC | Developer-tool plugin page for skills/rules: "All / User" segmented toggle, search, empty state with "Browse Marketplace", and two-column result rows. Very close domain match. |

## B. Search results filtered by source/publisher, with install state per item

| App | mobbin_url | image_url | What it lends |
|---|---|---|---|
| LangChain (Fleet Integrations) | https://mobbin.com/screens/c3ee2df7-4268-4daa-a45d-194770781ec6 | https://mobbin.com/api/mcp/short/zgs20eIE | Results grouped by **provider** (Google / Microsoft / Other) with per-row state: green "Connected" badge or outlined "Connect" button, plus a left filter list (All / Connected / categories / apps). Maps directly to "group by source". |
| Langdock (Agent templates) | https://mobbin.com/screens/863cf03c-fc11-487a-9ae6-cae7e8200fe8 | https://mobbin.com/api/mcp/short/dHkAQ6iU | Search field with a row of facet chips that carry counts ("Langdock 5", "Marketing 4"), one of which is a publisher. Shows chips narrowing a live query. |
| Langdock (Integrations, chips) | https://mobbin.com/screens/1220a80f-e373-46cb-91b8-c03e048f275b | https://mobbin.com/api/mcp/short/6n9kLnET | State-filter chips "All / Connected / Built by me / Needs configuration": the "Built by me" chip is a stand-in for "repositories I added". |
| Zoho CRM (Marketplace) | https://mobbin.com/screens/ddccef78-ab49-4882-8523-fad8d1788aa5 | https://mobbin.com/api/mcp/short/sS88dA6j | Row of filter dropdowns beside search, sidebar list of publishers (Zoho / Google / Microsoft…), and "All Extensions / Installed" tabs. Dropdown-style source filter. |
| PandaDoc (Marketplace) | https://mobbin.com/screens/1e8af85f-b098-4875-b922-7f18327fba42 | https://mobbin.com/api/mcp/short/t7OOGubF | Quiet "✓ Installed" footer on already-installed cards (no button), plus an "Installed add-ons" entry in the left nav. |
| Stripe (Apps overview, installing) | https://mobbin.com/screens/17989fff-020f-482d-9bc6-b0019b01b32d | https://mobbin.com/api/mcp/short/QZK1WC0D | "Installing {name}" spinner state after an inline Install link in a "Popular this month" list. Use as the Installing state, but inline rather than modal. |
| Zendesk (Marketplace, install done) | https://mobbin.com/screens/9e04008c-e863-451e-90b9-04adcd8225b4 | https://mobbin.com/api/mcp/short/6mOgA6py | Progress bar during install, then a "Success: Linear installed" toast. Lends the completion feedback. |

## C. Adding a custom source/repository by URL, and managing the list

| App | mobbin_url | image_url | What it lends |
|---|---|---|---|
| Sentry (GitHub repositories) | https://mobbin.com/screens/da54b970-6ad0-4355-8d49-d9a84e37d47c | https://mobbin.com/api/mcp/short/2VAOc9Bc | Bordered list of repositories (owner/name over URL link), trash icon per row, "Add Repository" header button, and an "added" toast. The simplest add/remove list. |
| v0 (Import from GitHub) | https://mobbin.com/screens/7dc55c69-d79b-4546-98a7-10dab6b0656e | https://mobbin.com/api/mcp/short/QGOKLmyS | Small dialog: "Import from a URL" field with an inline primary button, then an optional account picker with a repo search below. |
| Google Gemini (Import code) | https://mobbin.com/screens/a6046929-212e-4a94-9694-3296eb311a36 | https://mobbin.com/api/mcp/short/n0LG65Iq | URL field with an inline spinner while validating, then a parsed preview ("Repository: owner/name, Branch: default") before confirming. Good model for validating a Git URL. |
| Arcade (Add URL) | https://mobbin.com/screens/513b5030-d8fe-4ce6-86df-4494c2e23b69 | https://mobbin.com/api/mcp/short/72kfot43 | Minimal "Add URL" dialog: one field, helper line, Cancel / Add URL footer. The baseline dialog shape. |
| Cofounder (Repositories settings) | https://mobbin.com/screens/65e469a8-2efe-4155-b549-ecae8ae02b29 | https://mobbin.com/api/mcp/short/4tSvXKq1 | Settings page with "Active repositories" cards tagged "Managed repo", plus a "Connect existing repo" action. Shows tagging curated vs user-owned sources. |
| Claude Code (Connect with GitHub) | https://mobbin.com/screens/764751b1-d951-4e21-8b48-48dcff556d25 | https://mobbin.com/api/mcp/short/3O8z2DgL | Two-option card for private-repo access (personal access token vs sign in) with "Skip this step". Relevant if private repositories need credentials. |

## D. "New" badge on newly added list items

| App | mobbin_url | image_url | What it lends |
|---|---|---|---|
| Superlist (Lists) | https://mobbin.com/screens/4266f255-ac95-4dad-99bc-40ad3a4bdadc | https://mobbin.com/api/mcp/short/qfLoUQAm | "Lists 1 new" count in the sidebar, and the new item shown first under a "New list" heading with "· 2 min ago". Lends both a nav-level hint and a row-level hint. |
| Wrike (Recycle bin) | https://mobbin.com/screens/ebf050d3-56f1-47b1-b29d-3dd78788ed8a | https://mobbin.com/api/mcp/short/B7EMPpzl | Plain trailing "New" text label (accent color) at the end of each row in a dense list. The lightest-weight treatment. |
| Ferndesk (sidebar list) | https://mobbin.com/screens/a0c1907a-1051-4202-8423-035bee740352 | https://mobbin.com/api/mcp/short/UFixrOs6 | Small tinted "New" pill directly after the item name in a list. The classic inline badge. |
| Contractbook (Data fields library) | https://mobbin.com/screens/3a36e3ba-e1b8-4bf6-8638-d85f6d33da45 | https://mobbin.com/api/mcp/short/5MQtGSBO | Green "New" pill beside a sidebar entry named "library". Same pill placement as a Library row would use. |
| Copy.ai (Workflow table) | https://mobbin.com/screens/3675105b-4d88-49ac-a805-87fe260bd003 | https://mobbin.com/api/mcp/short/9QrR6WSx | "NEW" sits in a status column beside "PROCESSING" and "COMPLETE", so New is one status among others rather than a decoration. |
| Sentry (Issues) | https://mobbin.com/screens/d2e3901c-272a-401c-af50-648b1bd62ed5 | https://mobbin.com/api/mcp/short/THbFD3a4 | Muted "New" vs "Ongoing" caption under a row's trend cell. A subtler secondary-text variant. |

## Mobbin AI usage notice (verbatim)

---

**Your Mobbin AI usage is high this month**

Usage is unlimited during beta. If your usage stays at this level, you may need to purchase more credits once limits take effect. [View usage and plan options](https://mobbin.com/settings/billing-and-usage?utm_source=claude-code&utm_medium=ai-assistant&utm_source_platform=mcp)
