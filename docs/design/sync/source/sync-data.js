/* Sync prototype fixture and state: what the Sync page reads, and the actions it runs.
 *
 * SYNC.on(fn) -> off           re-render on every change
 * SYNC.s                       {state, running, device, remote, checked, local, incoming, history, picks}
 * SYNC.setState(id)            demo: 'synced'|'push'|'pull'|'both'|'conflict'|'offline'|'none'|'nogit'
 * SYNC.run(action)             'sync'|'push'|'pull'|'check' (spins ~1.2 s, then lands)
 * SYNC.discard(slug) SYNC.restore(slug, change) SYNC.rename(name) SYNC.branch(name)
 * SYNC.disconnect() SYNC.connect(url, branch) SYNC.pick(slug, side)
 * SYNC.KIND                    added|changed|removed -> {mark, label, tint}
 */
window.SYNC = (function () {
  'use strict';
  const listeners = new Set();
  const emit = () => listeners.forEach((fn) => fn());
  const KIND = {
    added: { mark: '+', label: 'Added', tint: 'success' },
    changed: { mark: '~', label: 'Changed', tint: 'info' },
    removed: { mark: '−', label: 'Removed', tint: 'danger' },
  };
  const OTHER = 'MacBook Air';
  const local = () => [
    { slug: 'pdf-tools', kind: 'added', why: 'Imported from Claude Code' },
    { slug: 'code-review', kind: 'changed', why: 'Updated from its source' },
    { slug: 'old-linter', kind: 'removed', why: 'Removed outside SkillBinder' },
  ];
  const incoming = () => ({
    device: OTHER, when: 'Today 11:40',
    skills: [
      { slug: 'frontend-design', kind: 'added', why: 'Installed from Explore' },
      { slug: 'sql-optimizer', kind: 'changed', why: 'Updated from its source' },
    ],
  });
  const history = () => [
    { id: 'c6', day: 'Today', time: '09:15', device: 'Studio Mac', skills: [
      { slug: 'vitest-runner', kind: 'changed', why: 'Updated from its source' },
      { slug: 'git-commit', kind: 'added', why: 'Installed from Explore' }] },
    { id: 'c5', day: 'Yesterday', time: '18:02', device: OTHER, skills: [
      { slug: 'brand-guidelines', kind: 'added', why: 'Imported from Cursor' },
      { slug: 'release-notes', kind: 'added', why: 'Imported from Cursor' },
      { slug: 'sql-optimizer', kind: 'added', why: 'Installed from Explore' },
      { slug: 'frontend-design', kind: 'changed', why: 'Reinstalled' },
      { slug: 'old-prompts', kind: 'removed', why: 'Removed outside SkillBinder' }] },
    { id: 'c4', day: 'Sat, Nov 22', time: '10:30', device: 'Studio Mac', skills: [
      { slug: 'code-review', kind: 'changed', why: 'Restored to the version of Nov 18' }] },
    { id: 'c3', day: 'Tue, Nov 18', time: '16:44', device: OTHER, skills: [
      { slug: 'code-review', kind: 'changed', why: 'Updated from its source' },
      { slug: 'vitest-runner', kind: 'added', why: 'Installed from Explore' }] },
    { id: 'c1', day: 'Wed, Nov 12', time: '14:03', device: null, skills: [
      'code-review', 'frontend-design', 'old-linter', 'old-prompts', 'api-designer', 'docker-compose',
      'k8s-manifests', 'mermaid-diagrams', 'regex-helper', 'shell-scripts', 'tailwind-ui', 'terraform-plan',
      'typescript-strict', 'web-a11y'].map((slug) => ({ slug, kind: 'added' })) },
  ];

  const s = {
    state: 'push', running: null, device: 'Studio Mac', checked: '2 min ago',
    remote: { url: 'git@github.com:ada/skills.git', web: 'github.com/ada/skills', branch: 'main' },
    local: local(), incoming: null, history: history(), picks: {},
  };

  function setState(id) {
    Object.assign(s, { state: id, running: null, picks: {}, local: [], incoming: null, history: history(),
      checked: id === 'offline' ? '3 h ago' : '2 min ago' });
    if (['push', 'both', 'conflict'].includes(id)) s.local = local();
    if (['pull', 'both', 'conflict'].includes(id)) s.incoming = incoming();
    if (id === 'conflict') s.incoming.skills.push({ slug: 'code-review', kind: 'changed', why: 'Updated from its source' });
    if (id === 'none' || id === 'nogit') s.remote = null;
    else if (!s.remote) s.remote = { url: 'git@github.com:ada/skills.git', web: 'github.com/ada/skills', branch: 'main' };
    emit();
  }

  /** Slugs changed on both sides: the user picks a side for each (D2). */
  const conflicts = () => !s.incoming ? [] : s.incoming.skills.map((x) => x.slug).filter((slug) => s.local.some((l) => l.slug === slug));
  const resolved = () => conflicts().every((slug) => s.picks[slug]);

  function land(action) {
    s.running = null;
    s.checked = 'just now';
    const pulled = (action === 'sync' || action === 'pull') && s.incoming;
    const pushed = (action === 'sync' || action === 'push') && s.local.length;
    if (pulled) {
      const skills = s.incoming.skills.filter((x) => s.picks[x.slug] !== 'local');
      s.history.unshift({ id: `in${Date.now()}`, day: 'Today', time: '11:40', device: s.incoming.device, skills, fresh: true });
      s.incoming = null;
    }
    if (pushed) {
      const skills = s.local.filter((x) => s.picks[x.slug] !== 'remote');
      if (skills.length) s.history.unshift({ id: `out${Date.now()}`, day: 'Today', time: 'Now', device: s.device, skills, fresh: true });
      s.local = [];
    }
    s.picks = {};
    s.state = s.local.length ? 'push' : s.incoming ? 'pull' : 'synced';
    emit();
    return { pulled: !!pulled, pushed: !!pushed };
  }
  function run(action) {
    if (s.running) return Promise.resolve(null);
    s.running = action;
    emit();
    return new Promise((done) => setTimeout(() => {
      if (action === 'check') {
        s.running = null;
        s.checked = 'just now';
        if (s.state === 'offline') s.state = 'synced';
        emit();
        return done({});
      }
      done(land(action));
    }, 1200));
  }

  function discard(slug) {
    s.local = s.local.filter((x) => x.slug !== slug);
    if (!s.local.length) s.state = s.incoming ? 'pull' : 'synced';
    emit();
  }
  function restore(slug, change) {
    s.local = s.local.filter((x) => x.slug !== slug);
    s.local.push({ slug, kind: 'changed', why: `Restored to the version of ${change.day}`, restored: true });
    if (s.state === 'synced') s.state = 'push';
    if (s.state === 'pull') s.state = 'both';
    emit();
  }

  return {
    s, KIND, OTHER, conflicts, resolved, setState, run, discard, restore,
    on: (fn) => (listeners.add(fn), () => listeners.delete(fn)),
    pick(slug, side) { s.picks[slug] = side; emit(); },
    rename(name) { s.device = name; emit(); },
    branch(name) { s.remote.branch = name; s.checked = 'just now'; emit(); },
    disconnect() { s.remote = null; s.state = 'none'; s.local = []; s.incoming = null; emit(); },
    connect(url, branch) {
      const web = url.replace(/^git@([^:]+):/, '$1/').replace(/^https?:\/\//, '').replace(/\.git$/, '');
      s.remote = { url, web, branch: branch || 'main' };
      s.state = 'push';
      s.local = local();
      emit();
    },
  };
})();
