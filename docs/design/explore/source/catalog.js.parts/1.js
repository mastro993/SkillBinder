/* Explore prototypes: the catalog fixture and the rules of docs/design/explore/decisions.md.
 * Load after the kit's data.js (it reads DATA.skills for the library). State is module-level,
 * so it carries across variants. Skill identity is the slug (unique across every source).
 *
 * window.CATALOG
 *  Skill    {slug, name, desc, creator (id), repo ('owner/repo'), path, url (opens in the browser),
 *            installsWeek (number | null outside the snapshot), files [paths],
 *            origin 'catalog'|'repo'|'skills.sh', repoId? (a user repository), private? bool}
 *  Creator  {id, initials, color (hex, for --tint), url, skillCount, installsWeek}
 *  Repo     {id, name, owner, full ('owner/name'), url, private, skillCount, slugs}
 *  settings {git, online, cache, access, remoteMs: 900, installMs: 1200}  demo conditions;
 *           setSetting(key, value) changes one and emits {type:'settings'}.
 *  Lookups  skill(slug) creator(id) repo(id) creators() -> ranked by summed installsWeek (T2)
 *           creatorSkills(id) repoSkills(id) repositories() -> Repo[] in added order
 *           trending(n = 12) -> catalog skills by installsWeek (T1)
 *           sources() -> [{id:'creator:<id>'|'repo:<id>'|'skills.sh', label, kind, private?}]  (C3)
 *           status() -> 'ok' | 'offline' (cached snapshot, no skills.sh) | 'unavailable' (no cache)
 *  Search   search(query, {source}) -> {local: Skill[], remote: Promise<Skill[]> | null, offline}
 *           local = snapshot + your repositories, at once, best match first (C2). remote =
 *           skills.sh after settings.remoteMs, never containing a slug already in local (append
 *           it below; never reorder). null when offline, for a creator/repo source, or an empty
 *           query. Empty query + a source lists that source by installs. Drop a stale remote
 *           yourself (compare the query when it resolves).
 *  Install  installState(slug) -> 'install'|'installing'|'in-library'|'collision'|'needs-git'|'failed'
 *           install(slug) -> Promise<{ok, error?}> after settings.installMs; several run at once (I2).
 *             Fails for a private repository's skill while settings.access is false (I6).
 *           installError(slug) -> "Couldn't install. Check your access to <repo>." | null
 *           collisionTip(slug) -> "Your library has another skill named pdf" (I4)
 *           installable(repoId) -> slugs not in library yet; installAll(repoId) -> Promise (R3)
 *  Library  librarySkills() -> [{slug, desc, source, isNew}] the kit's 18 skills + the catalog's;
 *           inLibrary(slug) isNew(slug) markSeen(slug) (N1: activated or its page opened)
 *  Repos    listRepository(url) -> Promise<{state, repo?, skills?}> after ~1.1 s ("listing" while
 *             pending). state: 'found' | 'no-skills' | 'unreachable' | 'already-added' | 'invalid'.
 *             Try: github.com/trailofbits/skills (found 8), github.com/acme-corp/empty-repo
 *             (no skills), git@gitlab.internal:team/skills.git (unreachable), either fixture URL.
 *           addRepository(url) -> Promise<same> (lists, then adds when found; R1)
 *           removeRepository(id) -> undo()   (installed skills stay; R4)
 *  Text     skillMd(slug) -> Promise<string> (~400 ms; P1)  fmtInstalls(12400) -> '12.4k'
 *           installsLabel(skill) -> '+12.4k this week' | ''   sourceLabel(skill) -> creator id |
 *           repository name | 'skills.sh'
 *  Events   on(fn) -> unsubscribe; fn({type:'install'|'installed'|'failed'|'repo'|'settings'|'seen', slug?})
 *           reset()
 */
(function () {
  'use strict';
  // [id, initials, color, [[repo, pathPrefix, [[slug, desc, installsWeek], ...]], ...]]
  const CREATORS = [
    ['anthropics', 'AN', '#d97757', [['skills', 'skills/', [
      ['pdf', 'Extract text and tables, merge files, and fill PDF forms.', 18300],
      ['docx', 'Create and edit Word documents with tracked changes.', 12100],
      ['xlsx', 'Build spreadsheets with formulas, formatting, and charts.', 9800],
      ['pptx', 'Create and edit slide decks and speaker notes.', 8700],
      ['frontend-design', 'Distinctive, intentional visual design for web UIs.', 31200],
      ['skill-creator', 'Create, test, and improve agent skills.', 15600],
      ['mcp-builder', 'Build MCP servers with well-designed tools.', 7400],
      ['webapp-testing', 'Test local web apps with Playwright and screenshots.', 6100],
      ['canvas-design', 'Make posters and visual art as PNG and PDF.', 4300],
      ['algorithmic-art', 'Seeded generative art with p5.js.', 3900],
      ['brand-guidelines', 'Apply brand colors and type to artifacts.', 2100],
      ['internal-comms', 'Write status reports and updates in house style.', 2800],
      ['theme-factory', 'Style slides and docs with ready-made themes.', 3300],
      ['doc-coauthoring', 'Co-write docs through drafts and reader tests.', 2500],
      ['web-artifacts-builder', 'Build multi-part HTML artifacts with React and Tailwind.', 5200]]]]],
    ['vercel-labs', 'VL', '#737373', [['skills', 'skills/', [
      ['find-skills', 'Find and install agent skills for the task at hand.', 48200]]],
    ['agent-skills', 'skills/', [
      ['react-best-practices', 'React and Next.js performance rules from Vercel.', 28400],
      ['web-design-guidelines', 'Review UI code against web interface guidelines.', 22100],
      ['composition-patterns', 'React composition patterns that scale.', 9600],
      ['react-native-guidelines', 'Performance and UX rules for React Native.', 6800],
      ['deploy-to-vercel', 'Deploy the current project and share the URL.', 5400]]]]],
    ['mattpocock', 'MP', '#3b82f6', [['skills', 'skills/', [
      ['tdd', 'Red-green-refactor with integration-style tests.', 11800],
      ['grilling', 'Stress-test a plan by questioning it relentlessly.', 7200],
      ['prototype', 'Build a throwaway prototype to answer a design question.', 5100],
      ['domain-modeling', 'Sharpen the glossary and record decisions.', 4700],
      ['codebase-design', 'A shared vocabulary for designing deep modules.', 3600],
      ['diagnosing-bugs', 'A diagnosis loop for hard bugs and regressions.', 6900],
      ['writing-for-agents', 'Write skills and AGENTS.md files agents follow.', 4100],
      ['wizard', 'Generate a bash wizard for steps only you can do.', 1900]]]]],
    ['obra', 'OB', '#f59e0b', [['superpowers', 'skills/', [
      ['brainstorming', 'Turn rough ideas into designs through questions.', 19400],
      ['writing-plans', 'Write implementation plans with bite-sized tasks.', 13200],
      ['executing-plans', 'Execute a written plan in batches with checkpoints.', 10900],
      ['systematic-debugging', 'Find the root cause before any fix.', 12600],
      ['test-driven-development', 'Write the test first, watch it fail, then pass it.', 11300],
      ['subagent-driven-development', 'One fresh subagent per task, with review between.', 8400],
      ['using-git-worktrees', 'Isolate feature work in git worktrees.', 6200],
      ['verification-before-completion', 'Run the checks before calling work done.', 7700],
      ['requesting-code-review', 'Ask for review against the plan before merging.', 5600],
      ['receiving-code-review', 'Weigh review feedback with rigor.', 4800],
      ['finishing-a-development-branch', 'Merge, open a PR, or clean up when done.', 4400],
      ['dispatching-parallel-agents', 'Split independent problems across agents.', 5900],
      ['writing-skills', 'Write and test skills with pressure scenarios.', 3700]]]]],
    ['openai', 'OA', '#10a37f', [['skills', 'skills/.curated/', [
      ['gh-fix-ci', 'Find failing GitHub Actions checks and fix them.', 8900],
      ['gh-address-comments', 'Address review comments on the open pull request.', 6400],
      ['playwright', 'Drive a real browser from the terminal.', 7800],
      ['figma-implement-design', 'Turn Figma frames into production code.', 5300],
      ['security-best-practices', 'Review code for language-specific security issues.', 4600]]]]],
    ['microsoft', 'MS', '#0078d4', [['skills', 'skills/', [
      ['e2e-playwright', 'Author and debug Playwright end-to-end tests.', 9100],
      ['azure-functions', 'Build and deploy Azure Functions apps.', 3200],
      ['azure-cosmos-db', 'Model data and write queries for Cosmos DB.', 2400],
      ['typescript-sdk-guidelines', 'Design TypeScript SDKs the Azure way.', 1600]]]]],
    ['supabase', 'SU', '#3ecf8e', [['agent-skills', 'skills/', [
      ['supabase-postgres-best-practices', 'Postgres performance and security rules.', 14700],
      ['supabase-auth', 'Add sign-in, sessions, and row-level security.', 4900],
      ['supabase-edge-functions', 'Write and deploy Edge Functions with Deno.', 3800],
      ['supabase-realtime', 'Subscribe to database changes and broadcast events.', 2200]]]]],
    ['cloudflare', 'CF', '#f38020', [['skills', 'skills/', [
      ['workers-best-practices', 'Write fast, safe Cloudflare Workers.', 6700],
      ['durable-objects', 'Model stateful coordination with Durable Objects.', 4200],
      ['wrangler', 'Configure, run, and deploy projects with Wrangler.', 5800],
      ['agents-sdk', 'Build stateful agents on the Agents SDK.', 3500]]]]],
    ['expo', 'EX', '#4630eb', [['skills', 'plugins/expo/skills/', [
      ['building-native-ui', 'Build native-feeling UI with Expo Router.', 7100],
      ['expo-deployment', 'Ship to the App Store and Play Store with EAS.', 4500],
      ['upgrading-expo', 'Upgrade Expo SDK versions and fix breaking changes.', 3900],
      ['native-data-fetching', 'Fetch, cache, and sync data in Expo apps.', 2700],
      ['expo-dev-client', 'Build and use development clients.', 1800]]]]],
    ['enterprise-developer-experience-collective', 'ED', '#db2777', [['agent-skills-for-large-organizations', 'skills/', [
      ['architecture-decision-records-reviewer-for-large-monorepos', 'Review ADRs across many teams and flag decisions that conflict with each other before they ship.', 1200],
      ['quarterly-dependency-upgrade-planner', 'Plan dependency upgrades across services by risk.', 640],
      ['incident-timeline-builder', 'Assemble incident timelines from logs and chat.', 420]]]]],
  ];

  // Your repositories (R2: this device only): [id, url, private, [[slug, desc], ...]]
  const REPOS = [
    ['acme-internal', 'git@github.com:acme-corp/internal-skills.git', true, [
      ['acme-deploy', 'Deploy services through the Acme release train.'],
      ['acme-oncall', 'Triage pages with the on-call runbooks.'],
      ['incident-postmortem', 'Write blameless postmortems in the Acme template.'],
      ['acme-design-tokens', 'Use Acme design tokens instead of raw values.'],
      ['api-style-guide', 'Name and version REST endpoints the Acme way.'],
      ['feature-flags', 'Add, roll out, and retire LaunchDarkly flags.'],
      ['data-retention-policy', 'Check new tables against the retention policy.'],
      ['sso-integration', 'Wire a service into Acme single sign-on.'],
      ['billing-events', 'Emit billing events with the right schema.'],
      ['analytics-taxonomy', 'Name analytics events from the shared taxonomy.'],
      ['mobile-release-checklist', 'Walk the mobile release checklist.'],
      ['terraform-modules', 'Compose infrastructure from approved modules.'],
      ['grpc-conventions', 'Write protobufs that follow Acme conventions.'],
      ['code-owners', 'Keep CODEOWNERS accurate as teams move.'],
      ['acme-security-review', 'Run the security review before launch.']]],
    ['sanity-toolkit', 'https://github.com/sanity-io/agent-toolkit', false, [
      ['sanity-best-practices', 'Structure Sanity Studio projects well.'],
      ['groq-queries', 'Write and optimize GROQ queries.'],
      ['sanity-schema', 'Model content types and validation rules.'],
      ['portable-text', 'Render and transform Portable Text.'],
      ['content-migration', 'Migrate documents with the Sanity CLI.'],
      ['visual-editing', 'Set up visual editing and live preview.']]],
  ];
  // A repository listRepository() finds (the Add repository demo).
  const ADDABLE = { 'trailofbits/skills': [
    ['semgrep-rules', 'Write and test Semgrep rules.'], ['codeql-queries', 'Write CodeQL queries for a codebase.'],
    ['fuzzing-harness', 'Write fuzzing harnesses for parsers.'], ['audit-context', 'Build context before a security audit.'],
    ['constant-time-check', 'Spot timing side channels in crypto code.'], ['sarif-triage', 'Triage SARIF findings by severity.'],
    ['solidity-review', 'Review Solidity contracts for known bugs.'], ['supply-chain-audit', 'Audit dependencies for risk.']] };

  // skills.sh long tail: [slug, owner/repo, desc, installsWeek]
  const LONG_TAIL = [
    ['pdf-to-markdown', 'kenji-ito/agent-skills', 'Convert PDFs to clean Markdown with tables.', 310],
    ['invoice-parser', 'finlab-dev/skills', 'Pull totals and line items from invoice PDFs.', 120],
    ['pdf-form-filler', 'paperworks/skills', 'Fill PDF forms from JSON data.', 95],
    ['tailwind-v4-migration', 'ui-guild/skills', 'Migrate Tailwind v3 configs to v4.', 870],
    ['query-patterns', 'data-layer/skills', 'TanStack Query patterns for caching and mutations.', 540],
    ['postgres-index-advisor', 'dbtools-oss/skills', 'Suggest indexes from slow query logs.', 260],
    ['git-bisect-helper', 'cli-crafts/skills', 'Bisect regressions with a scripted test.', 150],
    ['release-drafter', 'shipit-labs/skills', 'Draft release notes from merged pull requests.', 340],
    ['swiftui-previews', 'ios-dev-kit/skills', 'Write SwiftUI previews for every state.', 410],
    ['rust-error-handling', 'ferris-works/skills', 'Idiomatic errors with thiserror and anyhow.', 690],
    ['docker-slim-images', 'containerly/skills', 'Shrink Docker images with multi-stage builds.', 230],
    ['a11y-color-contrast', 'inclusive-ui/skills', 'Check color pairs against WCAG contrast.', 180],
    ['openapi-client-gen', 'apiforge/skills', 'Generate typed API clients from OpenAPI specs.', 290],
    ['react-testing-recipes', 'testbench-io/skills', 'Testing Library recipes for React components.', 470],
  ];

  // Library sources for catalog slugs (the rest of the kit's library has no source: imported).
  // isNew: installed from Explore on this device and not yet activated or opened (N1).
  const LIBRARY = {
    'frontend-design': { source: 'anthropics/skills' },
    'e2e-playwright': { source: 'microsoft/skills' },
    'acme-deploy': { source: 'acme-corp/internal-skills' },
    'pdf': { source: null, desc: 'Fill and merge PDFs (your own copy).' },
    'skill-creator': { source: 'anthropics/skills', isNew: true },
    'tdd': { source: 'mattpocock/skills', isNew: true },
    'supabase-postgres-best-practices': { source: 'supabase/agent-skills', isNew: true },
  };

  const FILES = {
    'pdf': ['SKILL.md', 'forms.md', 'reference.md', 'scripts/fill_form.py', 'scripts/extract_text.py', 'LICENSE.txt'],
    'frontend-design': ['SKILL.md', 'LICENSE.txt'],
    'skill-creator': ['SKILL.md', 'references/schemas.md', 'scripts/init_skill.py', 'scripts/package_skill.py', 'eval-viewer/viewer.html', 'LICENSE.txt'],
    'find-skills': ['SKILL.md'],
    'brainstorming': ['SKILL.md', 'visual-companion.md'],
  };
  const MD = {
    'pdf': '# PDF processing\n\nUse pypdf to merge and split, pdfplumber to pull text and tables.\n\n## Fill a form\n1. Run `scripts/extract_text.py` to list the fields.\n2. Write the values as JSON.\n3. Run `scripts/fill_form.py form.pdf values.json`.\n\nSee forms.md for checkbox and radio fields.',
    'frontend-design': '# Frontend design\n\nBefore coding, pick a clear direction: purpose, tone, constraints, and the one thing a visitor remembers.\n\n- Typography with character, not the default stack\n- A dominant color with sharp accents\n- Motion where it earns attention\n\nMatch the effort to the vision.',
    'tdd': '# Test-driven development\n\n1. Write one failing test for the next behavior.\n2. Make it pass with the simplest change.\n3. Refactor while green.\n\nPrefer integration-style tests through public interfaces.',
    'brainstorming': '# Brainstorming ideas into designs\n\nUnderstand the idea through questions, one at a time. Offer 2-3 approaches with trade-offs, then present the design in short sections and check each one.',
    'find-skills': '# Find skills\n\nWhen the user asks for a capability, search the skills catalog first.\n\n```\nnpx skills find <query>\n```\n\nPresent the best matches with their install counts.',
  };

