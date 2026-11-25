/* Plugins prototypes: sample data and an in-memory store (decisions P1-P8, A1-A7). Load after shell.js.
 *
 * window.PLUGINS
 *   AGENTS          chosen plugin-capable agents, by name: Claude Code, Codex, Cursor, Droid, GitHub Copilot.
 *                   {id, name, logo, cmd?, scope?, hint?, count}; no cmd = no install command (P2: hint instead).
 *   list()          plugins for the current list mode, sorted by name (A5)
 *   plugin(id)      {id, name, mkt|null, repo|null, latest, desc, inst:{agentId:{v, off?}}, known:[agentIds], skills:[{slug, desc}]}
 *   mode            'normal' | 'empty' | 'many' | 'loading'   setMode(mode) resets the store
 *   inst(p, aid)    {v, off?} | null     behind(p, aid) -> bool (P8)     holders(p) -> agents that have it
 *   busy(pid, aid)  'install' | 'update' | 'uninstall' | null    error(pid, aid) -> string | null
 *   run(pid, aid, op)  runs the agent's command; settings.fail makes install fail with the agent's output (A3)
 *   commands(p, a)  the commands an install runs: marketplace add first when the agent does not know it (P6)
 *   library         library skills [{slug, desc, agents}]   alsoIn(slug) -> plugins shipping that slug (P3)
 *   readme(p)  skillMd(p, slug)  texts from plugins-text.js, or a short generated one
 *   on(fn) -> off   fn() after every change
 */
(function () {
  'use strict';
  const T = window.PL_TEXT || { readme: {}, skill: {} };

  // Retained Hugeicons (assets/hugeicons) the kit's generated block lacks, normalized as gen_icons.py does.
  Object.assign(window.ICONS, {
    puzzle: '<svg xmlns="http://www.w3.org/2000/svg" aria-hidden="true" viewBox="0 0 24 24" fill="none"><path d="M12.828 6.00096C12.9388 5.68791 12.999 5.35099 12.999 5C12.999 3.34315 11.6559 2 9.99904 2C8.34219 2 6.99904 3.34315 6.99904 5C6.99904 5.35099 7.05932 5.68791 7.17008 6.00096C4.88532 6.0093 3.66601 6.09039 2.87772 6.87868C2.08951 7.66689 2.00836 8.88603 2 11.1704C2.31251 11.06 2.64876 11 2.99904 11C4.6559 11 5.99904 12.3431 5.99904 14C5.99904 15.6569 4.6559 17 2.99904 17C2.64876 17 2.31251 16.94 2 16.8296C2.00836 19.114 2.08951 20.3331 2.87772 21.1213C3.66593 21.9095 4.88508 21.9907 7.16941 21.999C7.05908 21.6865 6.99904 21.3503 6.99904 21C6.99904 19.3431 8.34219 18 9.99904 18C11.6559 18 12.999 19.3431 12.999 21C12.999 21.3503 12.939 21.6865 12.8287 21.999C15.113 21.9907 16.3322 21.9095 17.1204 21.1213C17.9086 20.333 17.9897 19.1137 17.9981 16.829C18.3111 16.9397 18.648 17 18.999 17C20.6559 17 21.999 15.6569 21.999 14C21.999 12.3431 20.6559 11 18.999 11C18.648 11 18.3111 11.0603 17.9981 11.171C17.9897 8.88627 17.9086 7.66697 17.1204 6.87868C16.3321 6.09039 15.1128 6.0093 12.828 6.00096Z" stroke="currentColor" stroke-linejoin="round" stroke-width="1.5"/></svg>',
    refresh: '<svg xmlns="http://www.w3.org/2000/svg" aria-hidden="true" viewBox="0 0 24 24" fill="none"><path d="M20.0092 2V5.13219C20.0092 5.42605 19.6418 5.55908 19.4537 5.33333C17.6226 3.2875 14.9617 2 12 2C6.47715 2 2 6.47715 2 12C2 17.5228 6.47715 22 12 22C17.5228 22 22 17.5228 22 12" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5"/></svg>',
  });

  // Droid has no bundled logo: the square terminal glyph (AGENTS.md).
  const AGENTS = [
    { id: 'claude', name: 'Claude Code', logo: 'claude-code', cmd: 'claude', count: 14 },
    { id: 'codex', name: 'Codex', logo: 'codex', hint: "Install from Codex's /plugins", count: 9 },
    { id: 'cursor', name: 'Cursor', logo: 'cursor', hint: "Install from Cursor's plugins", count: 6 },
    { id: 'droid', name: 'Droid', logo: 'square-terminal', cmd: 'droid', scope: ' --scope user', count: 4 },
    { id: 'copilot', name: 'GitHub Copilot', logo: 'github-copilot', cmd: 'copilot', count: 7 },
  ];
  const agent = (id) => AGENTS.find((a) => a.id === id);

  const sk = (list) => list.map(([slug, desc]) => ({ slug, desc }));
  const OFFICIAL = 'claude-plugins-official';
  const claudeOnly = (v, extra = {}) => ({ inst: { claude: Object.assign({ v }, extra) }, known: ['claude'] });
  const BASE = [
    Object.assign({ id: 'fstack', mkt: 'fedemas', repo: 'mastro993/fstack', latest: '0.9.0',
      desc: 'Design, prototype, plan, and ship features with coding agents.',
      inst: { claude: { v: '0.9.0' }, copilot: { v: '0.8.0' } }, known: ['claude', 'copilot'] }, { skills: sk([
      ['feature-design', 'Shape a rough idea into an agreed spec'],
      ['ux-brainstorm', 'Prototype a screen or flow before building it'],
      ['domain-driven-design', 'Settle names and module boundaries'],
      ['design-system', 'Create, audit, or extend a design system'],
      ['create-issues', 'Turn agreed work into tracker issues'],
      ['comment-sweep', 'Clean up the comments in a branch'],
      ['pr-babysit', 'Watch a pull request through CI and review'],
      ['pr-ship', 'Land a change on main'],
      ['resolve-merge-conflicts', 'Resolve conflicts in a merge or rebase'],
      ['test-driven-development', 'Build behavior changes test-first'],
    ]) }),
    Object.assign({ id: 'mattpocock-skills', mkt: OFFICIAL, repo: 'mattpocock/skills', latest: '1.3.1',
      desc: "Matt Pocock's skills for planning, testing, and reviewing code." }, claudeOnly('1.3.1'), { skills: sk([
      ['tdd', 'Red-green-refactor, one test at a time'], ['grilling', 'Stress-test a plan with hard questions'],
      ['prototype', 'Build a throwaway prototype'], ['research', 'Research a question from primary sources'],
      ['domain-modeling', 'Sharpen the domain model and glossary'], ['codebase-design', 'Design deep modules'],
      ['code-review', 'Review changes against standards and spec'], ['pr', 'Write a pull request body'],
      ['wizard', 'Walk a human through manual steps'], ['writing-for-agents', 'Write skills and agent docs'],
      ['diagnosing-bugs', 'Diagnose hard bugs and regressions'], ['triage', 'Sort incoming issues'],
    ]) }),
    Object.assign({ id: 'caveman', mkt: 'caveman', repo: 'JuliusBrussee/caveman', latest: '600e8ef',
      desc: 'Ultra-compressed replies that keep technical accuracy.' }, claudeOnly('600e8ef'), { skills: sk([
      ['caveman', 'Compressed replies at three levels'], ['caveman-commit', 'Terse Conventional Commits messages'],
      ['caveman-review', 'One-line review comments'], ['caveman-help', 'Quick reference for caveman modes'],
      ['compress', 'Compress memory files to save tokens'],
    ]) }),
    Object.assign({ id: 'beautiful-rust', mkt: 'beautiful-rust', repo: 'fedemas/beautiful-rust', latest: '1.4.0',
      desc: 'Write, port, and review idiomatic Rust.' }, claudeOnly('1.4.0'), { skills: sk([
      ['rust-best-practices', 'Write idiomatic, maintainable Rust'], ['rust-port', 'Port code to idiomatic Rust'],
      ['rust-review', 'Review Rust with cited fixes'],
    ]) }),
    Object.assign({ id: 'frontend-design', mkt: OFFICIAL, repo: 'anthropics/claude-plugins-official', latest: '1.1.1',
      desc: 'Distinctive, intentional visual design for new UI.' }, claudeOnly('1.1.1', { off: true }), { skills: sk([
      ['frontend-design', 'Guidance for distinctive visual design'],
    ]) }),
    Object.assign({ id: 'pr-review-toolkit', mkt: OFFICIAL, repo: 'anthropics/claude-plugins-official', latest: '1.0.1',
      desc: 'Review agents for pull requests.' }, claudeOnly('1.0.1'), { skills: [] }),
    Object.assign({ id: 'context7', mkt: OFFICIAL, repo: 'upstash/context7', latest: null,
      desc: 'Current library docs through an MCP server.' }, claudeOnly(null), { skills: [] }),
    { id: 'motion', mkt: null, repo: null, latest: '1.2.0', desc: 'Animate React interfaces with Motion.',
      inst: { cursor: { v: '1.2.0' } }, known: [], skills: sk([['motion-animations', 'Springs, keyframes, and exit animations'],
        ['motion-layout', 'Shared layout and reorder animations']]) },
  ];
  const LONG = { id: 'enterprise-observability-and-incident-response-toolkit', mkt: 'acme-internal-marketplace', repo: 'acme/agent-plugins',
    latest: '2.14.0', desc: 'Dashboards, alerts, runbooks, and postmortems for on-call teams at Acme.',
    inst: { claude: { v: '2.14.0' }, copilot: { v: '2.14.0' }, droid: { v: '2.12.0' } }, known: ['claude', 'copilot', 'droid'],
    skills: sk([['oncall-handoff', 'Summarize the shift for the next on-call'], ['incident-timeline', 'Build an incident timeline'],
      ['postmortem', 'Draft a blameless postmortem'], ['runbook-check', 'Check a runbook against the service'],
      ['slo-report', 'Report SLO burn'], ['alert-tuning', 'Tune noisy alerts'], ['log-query', 'Write log queries']]) };
  const EXTRA = [['playwright', 'Browser automation and end-to-end tests.', 2], ['sentry', 'Read and triage Sentry issues.', 1],
    ['linear', 'Plan and update Linear issues.', 0], ['vercel', 'Deploy and inspect Vercel projects.', 3],
    ['supabase', 'Manage Supabase databases and auth.', 2], ['stripe', 'Build and test Stripe integrations.', 1],
    ['figma', 'Read designs and tokens from Figma.', 1], ['github', 'Work with GitHub issues and pull requests.', 0]]
    .map(([id, desc, n]) => Object.assign({ id, mkt: OFFICIAL, repo: 'anthropics/claude-plugins-official', latest: '1.0.0', desc },
      claudeOnly('1.0.0'), { skills: Array.from({ length: n }, (_, i) => ({ slug: `${id}-${['guide', 'debug', 'deploy'][i]}`, desc: desc })) }));

  // Library skills (P3: a slug a plugin also ships gets the mark).
  const library = [
    ['caveman', 'Ultra-compressed replies that keep technical accuracy', ['claude', 'cursor']],
    ['caveman-commit', 'Terse Conventional Commits messages', ['claude']],
    ['changelog', 'Write release notes from merged pull requests', ['claude', 'codex']],
    ['k8s-debug', 'Debug failing Kubernetes pods', ['claude']],
    ['pdf-tools', 'Fill, merge, and split PDF files', ['claude', 'cursor', 'codex']],
    ['rust-best-practices', 'Write idiomatic, maintainable Rust', ['claude', 'copilot']],
    ['storybook', 'Write stories for UI components', ['cursor']],
    ['tailwind-helper', 'Tailwind classes from design tokens', ['cursor', 'copilot']],
  ].map(([slug, desc, agents]) => ({ slug, desc, agents }));

  const clone = (x) => JSON.parse(JSON.stringify(x));
  const subs = new Set();
  const emit = () => subs.forEach((fn) => fn());
  const settings = { fail: false, ms: 1400 };
  let all = [];
  let busy = {};
  let errors = {};
  const key = (pid, aid) => `${pid}:${aid}`;
  const P = {
    AGENTS, agent, settings, library, mode: 'normal',
    on(fn) { subs.add(fn); return () => subs.delete(fn); },
    setMode(mode) {
      P.mode = mode;
      all = clone(mode === 'many' ? BASE.concat(LONG, EXTRA) : BASE).map((p) => Object.assign(p, { name: p.id }));
      busy = {};
      errors = {};
      settings.fail = false;
      emit();
    },
    list() {
      if (P.mode === 'empty' || P.mode === 'loading') return [];
      return all.filter((p) => P.holders(p).length).sort((a, b) => a.name.localeCompare(b.name));
    },
    plugin: (id) => all.find((p) => p.id === id),
    inst: (p, aid) => p.inst[aid] || null,
    behind(p, aid) { const i = p.inst[aid]; return !!(i && i.v && p.latest && i.v !== p.latest); },
    holders: (p) => AGENTS.filter((a) => p.inst[a.id]),
    busy: (pid, aid) => busy[key(pid, aid)] || null,
    error: (pid, aid) => errors[key(pid, aid)] || null,
    setError(pid, aid, text) { errors[key(pid, aid)] = text; emit(); },
    commands(p, a) {
      const add = p.known.includes(a.id) ? [] : [`${a.cmd} plugin marketplace add ${p.repo}`];
      return add.concat(`${a.cmd} plugin install ${p.name}@${p.mkt}${a.scope || ''}`);
    },
    failText: (p, a) => `${a.cmd}: failed to add marketplace ${p.mkt}: git clone https://github.com/${p.repo} exited with 128: Could not resolve host: github.com`,
    run(pid, aid, op) {
      const p = P.plugin(pid);
      const a = agent(aid);
      const k = key(pid, aid);
      busy[k] = op;
      delete errors[k];
      emit();
      setTimeout(() => {
        delete busy[k];
        if (op === 'install' && settings.fail) errors[k] = P.failText(p, a);
        else if (op === 'install') { p.inst[aid] = { v: p.latest }; if (!p.known.includes(aid)) p.known.push(aid); }
        else if (op === 'update') p.inst[aid].v = p.latest;
        else delete p.inst[aid];
        emit();
      }, op === 'install' ? settings.ms : settings.ms * 0.8);
    },
    alsoIn: (slug) => P.list().filter((p) => p.skills.some((s) => s.slug === slug)),
    readme: (p) => T.readme[p.id] || `# ${p.name}\n\n${p.desc}\n\n## Skills\n\n${p.skills.length
      ? p.skills.map((s) => `- \`${s.slug}\`: ${s.desc}`).join('\n') : 'This plugin ships agents, commands, or MCP servers, and no skills.'}`,
    skillMd(p, slug) {
      const s = p.skills.find((x) => x.slug === slug) || { slug, desc: '' };
      return T.skill[`${p.id}/${slug}`] || `---\nname: ${slug}\ndescription: ${s.desc}\n---\n# ${slug}\n\n${s.desc}.`;
    },
  };
  P.setMode('normal');
  window.PLUGINS = P;
})();
