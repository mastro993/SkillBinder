/* SkillBinder prototype kit: in-memory fixture + pure helpers (decisions.md model).
 *
 * window.DATA
 *  Fixture (read-only arrays; state lives behind the helpers):
 *   targets   [{id, name, path, logo, members?, memberLogos?}]  your agents = home skill folders,
 *             in the app's order (Universal first, then by name): universal (layers glyph; members
 *             = the registry's ~/.agents/skills agents; the stack shows the first three), claude,
 *             cursor, windsurf. One toggle each.
 *   wsDirs    [{id, path, name, logo, tip}]  project skill folders the native fixture offers a
 *             workspace (its detected agents' directories), in the app's order: agents
 *             (.agents/skills, "Universal") first, then by name. A row is named after the detected
 *             agents that read it ("Claude Code, Cline, Cursor, fx, Warp" for .claude/skills) and
 *             shows the first one's logo; Cline's two own directories name their path.
 *   folders   ['Web', 'Backend', 'Writing']                     library folders (filter only)
 *   skills    [{slug, desc, folder|null, hasUpdate}]            18 library skills
 *   discoveryCount  2
 *  Lookups: skill(slug) target(id) wsDir(id) workspace(id) preset(id)
 *           presets() workspaces() activePreset() -> preset {id,name,color,skills}
 *  Holds ('all' = explicit "All agents", includes agents added later; array = exactly those):
 *   presetRaw(slug, presetId?) -> 'all' | ids | null      (default: active preset)
 *   presetHolds(slug, presetId?) -> resolved target ids
 *   wsRaw(wsId, slug) -> 'all' | dirIds | null
 *   workspaceHolds(wsId, slug) -> resolved dir ids ('all' = every folder the workspace uses)
 *   workspaceDirs(wsId) -> wsDir objects the workspace uses ([] for docs-site)
 *   isActive(slug)  whereActive(slug) -> {agents:[ids], workspaces:[{id, dirs:[ids]}]}
 *   agentSkills(targetId) -> slugs   workspaceSkills(wsId) -> slugs
 *   targetsLabel(raw, kind = 'agents'|'ws', wsId?) -> 'All agents' | 'Claude Code' | '2 agents' | ''
 *   isEdited(slug, place, wsId?) -> bool   placeName({target, ws?}) -> 'Claude Code' | 'acme-web · Universal'
 *  Filters: filterSkills({folder?, status:'all'|'active'|'inactive', updates?, query?}) -> skills
 *           counts({folder?, query?}) -> {all, active, inactive, updates}
 *           sidebarCounts() -> {discovery, library, presets:{id:n}, workspaces:{id:n}, agents:{id:n}}
 *  Mutators (emit 'change'; return undo() unless noted):
 *   setAgents(slug, ids|'all'|null)          edits the preset that is on; []/null removes
 *   removeAgent(slug, targetId)              'all' minus one becomes the explicit rest
 *   setWorkspaceDirs(wsId, slug, dirs|'all'|null)   adds new folders to the workspace
 *   removeWorkspaceDir(wsId, slug, dirId)
 *   planSwitch(presetId) -> plan (pure)       switchPreset(presetId) -> plan (applies it)
 *   deletePreset(presetId) -> plan|null       (the preset that is on switches to Default)
 *   addSkills({slugs, targets: ids|'all', ws?}) -> plan   (picker; merges, never removes)
 *   reset()                                   restores the fixture
 *  plan = {kind, title, toast, adds:[slugs], removes:[slugs],
 *          pairs:[{slug, target, ws?, op:'add'|'remove', problem?:'different'|'edited'|'unwritable'}]}
 *  Events: on(fn) -> unsubscribe; fn({type, ...}) after every mutation.
 */
(function () {
  'use strict';

  // Agents without a logo of their own (Dexto, Loaf, Sarvam Code) show the square terminal glyph.
  const targets = [
    {
      id: 'universal', name: 'Universal', path: '~/.agents/skills', logo: 'layers',
      members: ['Cline', 'Codex', 'Dexto', 'fx', 'Kimi Code CLI', 'Loaf', 'Pi', 'Sarvam Code', 'Warp', 'Zed'],
      memberLogos: ['cline', 'codex', 'square-terminal'],
    },
    { id: 'claude', name: 'Claude Code', path: '~/.claude/skills', logo: 'claude-code' },
    { id: 'cursor', name: 'Cursor', path: '~/.cursor/skills', logo: 'cursor' },
    { id: 'windsurf', name: 'Windsurf', path: '~/.codeium/windsurf/skills', logo: 'windsurf' },
  ];

  const wsDirs = [
    { id: 'agents', path: '.agents/skills', name: 'Universal', logo: 'layers',
      tip: 'Cline, Codex, Cursor, Dexto, fx, Kimi Code CLI, Loaf, Pi, Sarvam Code, Warp, Zed' },
    { id: 'claude', path: '.claude/skills', name: 'Claude Code, Cline, Cursor, fx, Warp', logo: 'claude-code', tip: '' },
    { id: 'cline', path: '.cline/skills', name: 'Cline (.cline/skills)', logo: 'cline', tip: '' },
    { id: 'clinerules', path: '.clinerules/skills', name: 'Cline (.clinerules/skills)', logo: 'cline', tip: '' },
    { id: 'codex', path: '.codex/skills', name: 'Codex, Cursor, fx, Warp', logo: 'codex', tip: '' },
    { id: 'cursor', path: '.cursor/skills', name: 'Cursor, Warp', logo: 'cursor', tip: '' },
    { id: 'dexto', path: '.dexto/skills', name: 'Dexto', logo: 'square-terminal', tip: '' },
    { id: 'fx', path: '.fx/skills', name: 'fx', logo: 'fx', tip: '' },
    { id: 'kimi-code', path: '.kimi-code/skills', name: 'Kimi Code CLI', logo: 'kimi-code-cli', tip: '' },
    { id: 'pi', path: '.pi/skills', name: 'Pi', logo: 'pi', tip: '' },
    { id: 'warp', path: '.warp/skills', name: 'Warp', logo: 'warp', tip: '' },
    { id: 'windsurf', path: '.windsurf/skills', name: 'Windsurf', logo: 'windsurf', tip: '' },
  ];

  const folders = ['Web', 'Backend', 'Writing'];

  const skills = [
    ['a11y-audit', 'Audit pages for WCAG accessibility issues.', 'Web'],
    ['api-docs', 'Generate OpenAPI reference docs from route handlers.', 'Backend'],
    ['changelog', 'Keep CHANGELOG.md in Keep a Changelog format.', 'Writing'],
    ['code-review', 'Review a diff for correctness bugs and style issues.', null],
    ['copy-editor', 'Edit product copy for clarity and tone.', 'Writing'],
    ['data-viz', 'Pick the right chart and build it accessibly.', 'Web'],
    ['docker-compose', 'Author and debug docker-compose files.', 'Backend'],
    ['e2e-playwright', 'Author and debug Playwright end-to-end tests.', 'Web'],
    ['frontend-design', 'Distinctive, intentional visual design for web UIs.', 'Web'],
    ['git-commit', 'Write concise Conventional Commits messages.', null],
    ['k8s-debug', 'Diagnose failing Kubernetes pods and services.', 'Backend', true],
    ['pdf-tools', 'Extract text, merge files, and fill PDF forms.', null, true],
    ['prompt-tuning', 'Tighten prompts and test them against examples.', null],
    ['release-notes', 'Draft release notes from merged pull requests.', 'Writing'],
    ['sql-helper', 'Write and explain SQL for Postgres and SQLite.', 'Backend'],
    ['storybook', 'Write Storybook stories for React components.', 'Web', true],
    ['tailwind-helper', 'Compose Tailwind classes and theme tokens.', 'Web'],
    ['test-writer', 'Write focused unit tests for the code you changed.', null],
  ].map(([slug, desc, folder, hasUpdate]) => ({ slug, desc, folder, hasUpdate: !!hasUpdate }));

  // Active only for your agents: changelog, git-commit, pdf-tools, release-notes, test-writer.
  // Only in workspaces: api-docs, docker-compose, e2e-playwright, sql-helper, storybook, tailwind-helper.
  // Both: code-review ('all' agents), frontend-design.
  // Inactive: a11y-audit, data-viz (Frontend only), k8s-debug (Backend only), copy-editor, prompt-tuning.
  const INITIAL = {
    activePreset: 'default',
    presets: [
      {
        id: 'default', name: 'Default', color: 'neutral',
        skills: {
          'code-review': 'all', 'git-commit': ['claude', 'universal'], 'pdf-tools': ['claude'],
          'test-writer': ['claude', 'cursor'], 'changelog': ['universal'],
          'frontend-design': ['claude', 'cursor'], 'release-notes': ['claude'],
        },
      },
      {
        id: 'frontend', name: 'Frontend', color: 'blue',
        skills: {
          'code-review': 'all', 'frontend-design': 'all', 'tailwind-helper': 'all',
          'storybook': ['claude', 'cursor'], 'a11y-audit': 'all', 'data-viz': ['cursor'],
        },
      },
      {
        id: 'backend', name: 'Backend', color: 'amber',
        skills: {
          'code-review': 'all', 'sql-helper': 'all', 'api-docs': ['universal'],
          'docker-compose': ['claude', 'universal'], 'k8s-debug': ['claude'],
        },
      },
    ],
    workspaces: [
      {
        id: 'acme-web', name: 'acme-web', path: '~/code/acme-web', dirs: ['agents', 'claude'],
        skills: {
          'code-review': 'all', 'storybook': ['claude'], 'e2e-playwright': 'all',
          'tailwind-helper': ['claude'], 'frontend-design': ['agents'],
        },
      },
      {
        id: 'api-server', name: 'api-server', path: '~/code/api-server', dirs: ['claude'],
        skills: { 'sql-helper': 'all', 'docker-compose': 'all', 'api-docs': ['claude'] },
      },
      { id: 'docs-site', name: 'docs-site', path: '~/code/docs-site', dirs: [], skills: {} },
    ],
  };

  // Copies the user edited in place ("You changed this skill here") and unmanaged
  // same-slug copies ("different copy already there"). Keys: slug@target or slug@ws/dir.
  const EDITED = new Set(['release-notes@claude', 'storybook@acme-web/claude']);
  const FOREIGN = new Set(['tailwind-helper@cursor']);

  const clone = (v) => JSON.parse(JSON.stringify(v));
  let state = clone(INITIAL);
  const listeners = new Set();

  const emit = (detail) => listeners.forEach((fn) => fn(detail));
  const bySlug = (a, b) => a.localeCompare(b);
  const placeKey = (slug, target, ws) => (ws ? `${slug}@${ws}/${target}` : `${slug}@${target}`);

  const skill = (slug) => skills.find((s) => s.slug === slug);
  const target = (id) => targets.find((t) => t.id === id);
  const wsDir = (id) => wsDirs.find((d) => d.id === id);
  const workspace = (id) => state.workspaces.find((w) => w.id === id);
  const preset = (id) => state.presets.find((p) => p.id === id);
  const activePreset = () => preset(state.activePreset);

  function presetRaw(slug, presetId = state.activePreset) {
    const raw = preset(presetId)?.skills[slug];
    return raw === undefined ? null : raw;
  }
  function presetHolds(slug, presetId) {
    const raw = presetRaw(slug, presetId);
    if (raw === 'all') return targets.map((t) => t.id);
    return raw ? targets.map((t) => t.id).filter((id) => raw.includes(id)) : [];
  }
  function wsRaw(wsId, slug) {
    const raw = workspace(wsId)?.skills[slug];
    return raw === undefined ? null : raw;
  }
  // A workspace's folders in the app's order (wsDirs), whatever order they were added in.
  const inDirOrder = (ids) => wsDirs.map((d) => d.id).filter((id) => ids.includes(id));
  function workspaceHolds(wsId, slug) {
    const ws = workspace(wsId);
    const raw = wsRaw(wsId, slug);
    if (!ws || !raw) return [];
    return inDirOrder(raw === 'all' ? ws.dirs : ws.dirs.filter((d) => raw.includes(d)));
  }
  const workspaceDirs = (wsId) => inDirOrder(workspace(wsId)?.dirs || []).map(wsDir);

  function whereActive(slug) {
    return {
      agents: presetHolds(slug),
      workspaces: state.workspaces
        .map((ws) => ({ id: ws.id, dirs: workspaceHolds(ws.id, slug) }))
        .filter((w) => w.dirs.length),
    };
  }
  function isActive(slug) {
    const where = whereActive(slug);
    return where.agents.length > 0 || where.workspaces.length > 0;
  }
  const agentSkills = (targetId) =>
    skills.map((s) => s.slug).filter((slug) => presetHolds(slug).includes(targetId));
  const workspaceSkills = (wsId) =>
    Object.keys(workspace(wsId)?.skills || {}).filter((slug) => workspaceHolds(wsId, slug).length).sort(bySlug);

