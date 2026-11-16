/* Scan and import prototypes: found-skill fixture and the rules of docs/specs/scan-and-import.md.
 *
 * window.FOUND
 *   skills        [{slug, desc, versions:[Version]}]  found skills not in library, in scan order
 *   Version       {id, modified (days ago), files, bytes, validation: 'valid'|'warnings'|'invalid'|
 *                  'blocked'|'empty', matchesSource, messages:[str], locations:[{path, readers}],
 *                  replaces:[place]}  place = an Agents row name or "workspace (agents)"
 *   inLibrary     3   found skills already in library (status line "N found · M already in library")
 *   MAX           500
 *   agentsOnly(skills) -> skills with only the locations in your agents' folders (onboarding)
 *   many(n)       -> n generated skills, for the 500 cap
 *   preferred(skill) -> version id   (matchesSource, else newest, else first)
 *   importable(v) -> not blocked and not empty      valid(v) -> valid or warnings
 *   initialTicks(skills, choice) -> Set of slugs: importable + valid chosen version, first 500
 *   status(skills) -> "N found · M already in library"
 *   size(v) -> "N files · 12.4 KB"   modified(v) -> "Modified 3 days ago"
 */
(function () {
  'use strict';
  const L = (path, readers) => ({ path, readers });
  const CLAUDE = L('~/.claude/skills', 'Claude Code');
  const UNIV = L('~/.agents/skills', 'Universal');
  const CURSOR = L('~/.cursor/skills', 'Cursor');
  const WINDSURF = L('~/.codeium/windsurf/skills', 'Windsurf');
  const ACME = L('~/code/acme-web/.claude/skills', 'acme-web (Claude Code)');
  const API = L('~/code/api-server/.claude/skills', 'api-server (Claude Code)');
  let n = 0;
  const v = (o) => Object.assign({ id: `v${++n}`, modified: 12, files: 3, bytes: 8200, validation: 'valid',
    matchesSource: false, messages: [], locations: [CLAUDE], replaces: [] }, o);
  const s = (slug, desc, versions) => ({ slug, desc, versions: versions.map(v) });

  const skills = [
    s('code-review', 'Reviews a diff for bugs, risky changes, and missing tests.', [{ modified: 3, files: 4, bytes: 14100, locations: [CLAUDE, UNIV], matchesSource: true }]),
    s('gh-stack', 'Manages stacked branches and pull requests with gh-stack.', [
      { modified: 2, files: 6, bytes: 21800, locations: [UNIV], matchesSource: true, replaces: ['Cursor'] },
      { modified: 41, files: 5, bytes: 19300, locations: [CURSOR] }]),
    s('caveman', 'Ultra-compressed replies that keep every technical detail.', [
      { modified: 5, files: 2, bytes: 6100, locations: [CLAUDE] },
      { modified: 90, files: 1, bytes: 2400, locations: [WINDSURF], validation: 'warnings', messages: ['SKILL.md has no description.'] }]),
    s('tdd', 'Test-driven development: red, green, refactor.', [{ modified: 8, locations: [CLAUDE, CURSOR] }]),
    s('pr-babysit', 'Watches a pull request through CI and review until it can merge.', [{ modified: 1, files: 5, bytes: 17600, locations: [CLAUDE], replaces: ['Universal'] }]),
    s('writing-for-agents', 'Writes skills, AGENTS.md, and CLAUDE.md files that agents follow.', [{ modified: 20, locations: [UNIV] }]),
    s('diagnosing-bugs', 'A diagnosis loop for hard bugs and performance regressions.', [{ modified: 14, locations: [CLAUDE] }]),
    s('rust-best-practices', 'Idiomatic, robust Rust: errors, traits, async, unsafe, and tests.', [{ modified: 6, files: 9, bytes: 48200, locations: [CLAUDE, UNIV] }]),
    s('api-docs', 'Generates reference docs from OpenAPI files.', [{ modified: 4, locations: [API], replaces: ['acme-web (Claude Code)'] }]),
    s('sql-helper', 'Explains and tunes slow SQL queries.', [{ modified: 30, locations: [API] }]),
    s('storybook', 'Writes Storybook stories for React components.', [{ modified: 10, locations: [ACME] }]),
    s('legacy-deploy', 'Deploys with the old Capistrano scripts.', [{ modified: 400, files: 2, bytes: 3100, locations: [WINDSURF], validation: 'invalid', messages: ['SKILL.md frontmatter is not valid YAML.'] }]),
    s('shell-tools', 'Runs helper binaries bundled with the skill.', [{ modified: 60, files: 4, bytes: 912000, locations: [CURSOR], validation: 'blocked', messages: ['Links outside the skill folder: bin/run → /usr/local/bin/run.'] }]),
    s('draft-notes', '', [{ modified: 2, files: 0, bytes: 0, locations: [CLAUDE], validation: 'empty', messages: ['The folder has no files.'] }]),
    s('internationalization-and-localization-string-extraction-helper', 'Finds hard-coded strings and moves them into translation catalogs for every locale the project ships.', [{ modified: 22, locations: [ACME] }]),
    s('docker-compose', 'Writes and debugs docker-compose files.', [{ modified: 9, locations: [API, ACME] }]),
    s('commit-message', 'Writes Conventional Commits messages from the staged diff.', [{ modified: 11, locations: [UNIV, CLAUDE] }]),
    s('release-notes', 'Drafts release notes from merged pull requests.', [{ modified: 17, locations: [CLAUDE] }]),
    s('tailwind-helper', 'Suggests Tailwind classes and spots conflicting ones.', [{ modified: 13, locations: [ACME] }]),
    s('security-review', 'Reviews pending changes for injection, secrets, and auth gaps.', [{ modified: 7, locations: [CLAUDE] }]),
    s('e2e-playwright', 'Writes Playwright end-to-end tests.', [{ modified: 25, locations: [ACME] }]),
    s('mermaid-diagrams', 'Draws flowcharts and sequence diagrams in Mermaid.', [{ modified: 33, locations: [UNIV] }]),
  ];

  const isAgents = (loc) => loc.path.startsWith('~/.');
  function agentsOnly(list) {
    return list.map((k) => ({ ...k, versions: k.versions.map((x) => ({ ...x, locations: x.locations.filter(isAgents),
      replaces: x.replaces.filter((p) => !p.includes('(')) })).filter((x) => x.locations.length) }))
      .filter((k) => k.versions.length);
  }
  function many(count) {
    const out = [];
    for (let i = 1; i <= count; i++) out.push(s(`skill-${String(i).padStart(3, '0')}`, `Generated skill ${i}.`, [{ modified: i % 60, locations: [i % 2 ? CLAUDE : UNIV] }]));
    return out;
  }
  const importable = (x) => x.validation !== 'blocked' && x.validation !== 'empty';
  const valid = (x) => x.validation === 'valid' || x.validation === 'warnings';
  function preferred(k) {
    const ok = k.versions.filter(importable);
    if (!ok.length) return k.versions[0].id;
    return (ok.find((x) => x.matchesSource) || [...ok].sort((a, b) => a.modified - b.modified)[0]).id;
  }
  function initialTicks(list, choice = {}) {
    const ticks = new Set();
    for (const k of list) {
      const x = k.versions.find((y) => y.id === (choice[k.slug] || preferred(k)));
      if (ticks.size < FOUND.MAX && importable(x) && valid(x)) ticks.add(k.slug);
    }
    return ticks;
  }
  const status = (list) => `${list.length} found · ${FOUND.inLibrary} already in library`;
  const kb = (b) => (b >= 1e6 ? `${(b / 1e6).toFixed(1)} MB` : `${(b / 1e3).toFixed(1)} KB`);
  const size = (x) => `${x.files} ${x.files === 1 ? 'file' : 'files'} · ${kb(x.bytes)}`;
  const ago = (d) => (d === 0 ? 'today' : d === 1 ? 'yesterday' : d < 30 ? `${d} days ago` : d < 365 ? (Math.round(d / 30) === 1 ? 'a month ago' : `${Math.round(d / 30)} months ago`) : 'over a year ago');
  const modified = (x) => `Modified ${ago(x.modified)}`;

  const FOUND = { skills, inLibrary: 3, MAX: 500, agentsOnly, many, preferred, importable, valid, initialTicks, status, size, modified };
  window.FOUND = FOUND;
})();
