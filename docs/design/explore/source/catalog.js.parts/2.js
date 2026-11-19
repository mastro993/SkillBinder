  // ---- index ---------------------------------------------------------------------------
  const DEFAULT_FILES = [['SKILL.md'], ['SKILL.md', 'reference.md'], ['SKILL.md', 'scripts/check.sh', 'examples.md']];
  const settings = { git: true, online: true, cache: true, access: true, remoteMs: 900, installMs: 1200 };
  const byInstalls = (a, b) => (b.installsWeek || 0) - (a.installsWeek || 0) || a.slug.localeCompare(b.slug);
  const index = new Map();
  const creators = [];
  let repos = [];
  let lib = {};
  let seen = new Set();
  const installing = new Set();
  const failed = new Map();
  const listeners = new Set();
  const emit = (e) => listeners.forEach((fn) => fn(e));

  function add(slug, desc, repo, path, o) {
    const sk = Object.assign({ slug, name: slug, desc, repo, path, creator: repo.split('/')[0], installsWeek: null,
      files: FILES[slug] || DEFAULT_FILES[slug.length % 3], origin: 'catalog' }, o);
    sk.url = sk.url || `https://github.com/${repo}/tree/main/${path}`;
    index.set(slug, sk);
    return sk;
  }
  function repoFromUrl(url) {
    const m = String(url).trim().match(/^(?:https?:\/\/|git@)?([\w.-]+)[/:]([\w.-]+)\/([\w.-]+?)(?:\.git)?\/?$/);
    return m ? { host: m[1], owner: m[2], name: m[3], full: `${m[2]}/${m[3]}` } : null;
  }
  function addRepo(id, url, priv, list) {
    const p = repoFromUrl(url);
    const r = { id, name: p.name, owner: p.owner, full: p.full, url, private: priv, slugs: [] };
    list.forEach(([slug, desc]) => {
      r.slugs.push(slug);
      add(slug, desc, p.full, `skills/${slug}`, { origin: 'repo', repoId: id, private: priv,
        url: priv ? `https://${p.host}/${p.full}` : `https://${p.host}/${p.full}/tree/main/skills/${slug}` });
    });
    r.skillCount = r.slugs.length;
    repos.push(r);
    return r;
  }
  CREATORS.forEach(([id, initials, color, repoList]) => {
    const c = { id, initials, color, url: `https://github.com/${id}`, slugs: [] };
    repoList.forEach(([repo, prefix, list]) => list.forEach(([slug, desc, n]) => {
      c.slugs.push(slug);
      add(slug, desc, `${id}/${repo}`, prefix + slug, { installsWeek: n });
    }));
    c.skillCount = c.slugs.length;
    c.installsWeek = c.slugs.reduce((t, s) => t + index.get(s).installsWeek, 0);
    creators.push(c);
  });
  LONG_TAIL.forEach(([slug, repo, desc, n]) => add(slug, desc, repo, `skills/${slug}`, { origin: 'skills.sh', installsWeek: n }));

  function reset() {
    repos.forEach((r) => r.slugs.forEach((s) => index.delete(s)));
    repos = [];
    REPOS.forEach((r) => addRepo(...r));
    lib = {};
    (window.DATA ? DATA.skills : []).forEach((s) => { lib[s.slug] = { slug: s.slug, desc: s.desc, source: null, isNew: false }; });
    Object.entries(LIBRARY).forEach(([slug, v]) => {
      lib[slug] = { slug, desc: v.desc || index.get(slug).desc, source: v.source, isNew: !!v.isNew };
    });
    seen = new Set();
    installing.clear();
    failed.clear();
    emit({ type: 'reset' });
  }
  reset();

  // ---- lookups ---------------------------------------------------------------------------
  const skill = (slug) => index.get(slug);
  const creator = (id) => creators.find((c) => c.id === id);
  const repo = (id) => repos.find((r) => r.id === id);
  const catalogSkills = () => [...index.values()].filter((s) => s.origin === 'catalog');
  const rankedCreators = () => creators.slice().sort((a, b) => b.installsWeek - a.installsWeek);
  const creatorSkills = (id) => catalogSkills().filter((s) => s.creator === id).sort(byInstalls);
  const repoSkills = (id) => (repo(id) ? repo(id).slugs.map(skill) : []);
  const trending = (n = 12) => catalogSkills().sort(byInstalls).slice(0, n);
  function status() {
    if (settings.online) return 'ok';
    return settings.cache ? 'offline' : 'unavailable';
  }
  function sources() {
    return [
      ...rankedCreators().map((c) => ({ id: `creator:${c.id}`, label: c.id, kind: 'creator' })),
      ...repos.map((r) => ({ id: `repo:${r.id}`, label: r.name, kind: 'repo', private: r.private })),
      { id: 'skills.sh', label: 'skills.sh', kind: 'skills.sh' },
    ];
  }
  function sourceLabel(s) {
    if (s.origin === 'repo') return repo(s.repoId) ? repo(s.repoId).name : s.repo;
    return s.origin === 'skills.sh' ? 'skills.sh' : s.creator;
  }

  // ---- search (C2, C3, C4) ----------------------------------------------------------------
  function score(s, q) {
    if (s.slug.startsWith(q)) return 0;
    if (s.slug.includes(q)) return 1;
    if (s.creator.includes(q) || s.repo.toLowerCase().includes(q)) return 2;
    return s.desc.toLowerCase().includes(q) ? 3 : -1;
  }
  function matches(list, q) {
    if (!q) return list.slice().sort(byInstalls);
    return list.map((s) => [s, score(s, q)]).filter(([, n]) => n >= 0)
      .sort((a, b) => a[1] - b[1] || byInstalls(a[0], b[0])).map(([s]) => s);
  }
  function search(query, { source } = {}) {
    const q = String(query || '').trim().toLowerCase();
    const [kind, id] = source ? source.split(':') : [];
    const offline = !settings.online;
    let pool = [];
    if (kind === 'creator') pool = catalogSkills().filter((s) => s.creator === id);
    else if (kind === 'repo') pool = repoSkills(id);
    else if (!source) pool = (settings.online || settings.cache ? catalogSkills() : []).concat(...repos.map((r) => repoSkills(r.id)));
    const local = q || source ? matches(pool, q) : [];
    const wantsRemote = q && !offline && (!source || kind === 'skills.sh');
    const shown = new Set(local.map((s) => s.slug));
    const remote = wantsRemote ? new Promise((done) => setTimeout(() => done(
      matches([...index.values()].filter((s) => s.origin === 'skills.sh' && !shown.has(s.slug)), q)), settings.remoteMs)) : null;
    return { query: q, source: source || null, local, remote, offline };
  }

  // ---- install (I1-I6, R3) -------------------------------------------------------------
  const inLibrary = (slug) => !!lib[slug];
  function installState(slug) {
    const s = skill(slug);
    if (lib[slug]) return s && lib[slug].source === s.repo ? 'in-library' : 'collision';
    if (installing.has(slug)) return 'installing';
    if (!settings.git) return 'needs-git';
    return failed.has(slug) ? 'failed' : 'install';
  }
  const installError = (slug) => failed.get(slug) || null;
  const collisionTip = (slug) => `Your library has another skill named ${slug}`;
  function install(slug) {
    const st = installState(slug);
    if (st !== 'install' && st !== 'failed') return Promise.resolve({ ok: false, state: st });
    const s = skill(slug);
    failed.delete(slug);
    installing.add(slug);
    emit({ type: 'install', slug });
    return new Promise((done) => setTimeout(() => {
      installing.delete(slug);
      if (s.private && !settings.access) {
        const error = `Couldn't install. Check your access to ${s.repo}.`;
        failed.set(slug, error);
        emit({ type: 'failed', slug });
        done({ ok: false, error });
        return;
      }
      lib[slug] = { slug, desc: s.desc, source: s.repo, isNew: true };
      emit({ type: 'installed', slug });
      done({ ok: true });
    }, settings.installMs));
  }
  const installable = (repoId) => repoSkills(repoId).filter((s) => !lib[s.slug] && !installing.has(s.slug)).map((s) => s.slug);
  const installAll = (repoId) => Promise.all(installable(repoId).map(install));

  // ---- library and New (N1, N2) ---------------------------------------------------------
  const librarySkills = () => Object.values(lib).sort((a, b) => a.slug.localeCompare(b.slug));
  const isNew = (slug) => !!(lib[slug] && lib[slug].isNew && !seen.has(slug));
  function markSeen(slug) {
    if (!isNew(slug)) return;
    seen.add(slug);
    emit({ type: 'seen', slug });
  }

  // ---- repositories (R1-R4) -----------------------------------------------------------
  const repoId = (p) => `${p.owner}-${p.name}`.toLowerCase();
  function listRepository(url) {
    const p = repoFromUrl(url);
    return new Promise((done) => setTimeout(() => {
      if (!p) return done({ state: 'invalid' });
      const have = repos.find((r) => r.full.toLowerCase() === p.full.toLowerCase());
      if (have) return done({ state: 'already-added', repo: have });
      if (!settings.online || /gitlab\.internal|unreachable/.test(url)) return done({ state: 'unreachable' });
      const list = ADDABLE[p.full] || (/empty/.test(p.name) ? [] : [
        [`${p.name}-conventions`, `Follow the ${p.name} conventions.`], [`${p.name}-setup`, `Set up ${p.name} locally.`],
        [`${p.name}-release`, `Release ${p.name} step by step.`]]);
      if (!list.length) return done({ state: 'no-skills' });
      done({ state: 'found', repo: { id: repoId(p), name: p.name, owner: p.owner, full: p.full, url, private: url.startsWith('git@') },
        skills: list.map(([slug, desc]) => ({ slug, desc })) });
    }, 1100));
  }
  function addRepository(url) {
    return listRepository(url).then((res) => {
      if (res.state !== 'found') return res;
      const r = addRepo(res.repo.id, url, res.repo.private, res.skills.map((s) => [s.slug, s.desc]));
      emit({ type: 'repo', id: r.id });
      return Object.assign(res, { repo: r });
    });
  }
  function removeRepository(id) {
    const at = repos.findIndex((r) => r.id === id);
    if (at < 0) return () => {};
    const [r] = repos.splice(at, 1);
    const kept = r.slugs.map((s) => [s, index.get(s)]);
    r.slugs.forEach((s) => index.delete(s));
    emit({ type: 'repo', id });
    return () => {
      repos.splice(at, 0, r);
      kept.forEach(([s, v]) => index.set(s, v));
      emit({ type: 'repo', id });
    };
  }

  // ---- text ------------------------------------------------------------------------------
  function fmtInstalls(n) {
    if (n == null) return '';
    if (n >= 1e6) return `${+(n / 1e6).toFixed(1)}M`;
    return n >= 1000 ? `${+(n / 1000).toFixed(1)}k` : String(n);
  }
  const installsLabel = (s) => (s && s.installsWeek ? `+${fmtInstalls(s.installsWeek)} this week` : '');
  function skillMd(slug) {
    const s = skill(slug);
    const body = MD[slug] || `# ${slug}\n\n${s.desc}\n\n## When to use\nAsk for it by name, or describe the task.`;
    return new Promise((done) => setTimeout(() => done(`---\nname: ${slug}\ndescription: ${s.desc}\n---\n\n${body}`), 400));
  }

  function setSetting(key, value) {
    settings[key] = value;
    emit({ type: 'settings', key });
  }

  window.CATALOG = {
    settings, setSetting, skill, creator, repo, creators: rankedCreators, creatorSkills, repoSkills,
    repositories: () => repos.slice(), trending, sources, status, sourceLabel, search,
    installState, install, installError, collisionTip, installable, installAll,
    librarySkills, inLibrary, isNew, markSeen, listRepository, addRepository, removeRepository,
    skillMd, fmtInstalls, installsLabel, reset,
    on(fn) { listeners.add(fn); return () => listeners.delete(fn); },
  };
})();
