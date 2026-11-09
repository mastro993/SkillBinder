  function targetsLabel(raw, kind = 'agents', wsId) {
    if (raw === 'all') return 'All agents';
    if (!raw || !raw.length) return '';
    if (raw.length > 1) return `${raw.length} agents`;
    return (kind === 'ws' ? wsDir(raw[0]) : target(raw[0]))?.name || raw[0];
  }
  const isEdited = (slug, place, wsId) => EDITED.has(placeKey(slug, place, wsId));
  function placeName({ target: place, ws }) {
    return ws ? `${ws} · ${wsDir(place)?.name || place}` : target(place)?.name || place;
  }

  function matches(s, { folder, status = 'all', updates, query }) {
    if (folder && s.folder !== folder) return false;
    if (updates && !s.hasUpdate) return false;
    if (status === 'active' && !isActive(s.slug)) return false;
    if (status === 'inactive' && isActive(s.slug)) return false;
    const q = (query || '').trim().toLowerCase();
    return !q || s.slug.includes(q) || s.desc.toLowerCase().includes(q);
  }
  const filterSkills = (opts = {}) => skills.filter((s) => matches(s, opts));
  function counts({ folder, query } = {}) {
    const base = { folder, query };
    return {
      all: filterSkills(base).length,
      active: filterSkills({ ...base, status: 'active' }).length,
      inactive: filterSkills({ ...base, status: 'inactive' }).length,
      updates: filterSkills({ ...base, updates: true }).length,
    };
  }
  function sidebarCounts() {
    const map = (list, fn) => Object.fromEntries(list.map((x) => [x.id, fn(x)]));
    return {
      discovery: 2,
      library: skills.length,
      presets: map(state.presets, (p) => Object.keys(p.skills).length),
      workspaces: map(state.workspaces, (w) => workspaceSkills(w.id).length),
      agents: map(targets, (t) => agentSkills(t.id).length),
    };
  }

  // ---- mutators -------------------------------------------------------------
  function writeRaw(holder, slug, value) {
    if (value === 'all') holder.skills[slug] = 'all';
    else if (value && value.length) holder.skills[slug] = [...value];
    else delete holder.skills[slug];
  }

  function setAgents(slug, value) {
    const p = activePreset();
    const prev = clone(p.skills[slug] ?? null);
    writeRaw(p, slug, value);
    emit({ type: 'agents', slug, preset: p.id });
    return () => { writeRaw(p, slug, prev); emit({ type: 'undo', slug }); };
  }
  function removeAgent(slug, targetId) {
    return setAgents(slug, presetHolds(slug).filter((id) => id !== targetId));
  }

  function setWorkspaceDirs(wsId, slug, value) {
    const ws = workspace(wsId);
    const prev = clone(ws.skills[slug] ?? null);
    const prevDirs = [...ws.dirs];
    if (Array.isArray(value)) value.forEach((d) => ws.dirs.includes(d) || ws.dirs.push(d));
    writeRaw(ws, slug, value);
    emit({ type: 'workspace', slug, ws: wsId });
    return () => { writeRaw(ws, slug, prev); ws.dirs = prevDirs; emit({ type: 'undo', slug, ws: wsId }); };
  }
  function removeWorkspaceDir(wsId, slug, dirId) {
    return setWorkspaceDirs(wsId, slug, workspaceHolds(wsId, slug).filter((d) => d !== dirId));
  }

  function problemFor(pair) {
    const key = placeKey(pair.slug, pair.target, pair.ws);
    if (pair.op === 'add' && FOREIGN.has(key)) return 'different';
    if (pair.op === 'remove' && EDITED.has(key)) return 'edited';
    return undefined;
  }
  function finishPlan(plan) {
    plan.pairs.forEach((p) => { const problem = problemFor(p); if (problem) p.problem = problem; });
    plan.adds = [...new Set(plan.pairs.filter((p) => p.op === 'add').map((p) => p.slug))].sort(bySlug);
    plan.removes = [...new Set(plan.pairs.filter((p) => p.op === 'remove').map((p) => p.slug))].sort(bySlug);
    return plan;
  }

  function planSwitch(presetId) {
    const to = preset(presetId);
    const pairs = [];
    for (const slug of skills.map((s) => s.slug)) {
      const before = presetHolds(slug);
      const after = presetHolds(slug, presetId);
      before.filter((t) => !after.includes(t)).forEach((t) => pairs.push({ slug, target: t, op: 'remove' }));
      after.filter((t) => !before.includes(t)).forEach((t) => pairs.push({ slug, target: t, op: 'add' }));
    }
    return finishPlan({ kind: 'switch', from: state.activePreset, to: presetId,
      title: `Switching to ${to.name}`, toast: `Switched to ${to.name}`, pairs });
  }
  function switchPreset(presetId) {
    const plan = planSwitch(presetId);
    state.activePreset = presetId;
    emit({ type: 'preset', preset: presetId });
    return plan;
  }
  function deletePreset(presetId) {
    if (presetId === 'default') return null;
    const wasOn = state.activePreset === presetId;
    const plan = wasOn ? planSwitch('default') : null;
    if (plan) Object.assign(plan, { kind: 'delete', title: 'Switching to Default', toast: 'Switched to Default' });
    if (wasOn) state.activePreset = 'default';
    state.presets = state.presets.filter((p) => p.id !== presetId);
    emit({ type: 'preset-deleted', preset: presetId });
    return plan;
  }

  function addSkills({ slugs, targets: chosen, ws }) {
    const pairs = [];
    for (const slug of slugs) {
      const before = ws ? workspaceHolds(ws, slug) : presetHolds(slug);
      const raw = ws ? wsRaw(ws, slug) : presetRaw(slug);
      const next = chosen === 'all' || raw === 'all' ? 'all' : [...new Set([...(raw || []), ...chosen])];
      const holder = ws ? workspace(ws) : activePreset();
      if (ws && Array.isArray(next)) next.forEach((d) => holder.dirs.includes(d) || holder.dirs.push(d));
      writeRaw(holder, slug, next);
      const after = ws ? workspaceHolds(ws, slug) : presetHolds(slug);
      after.filter((t) => !before.includes(t)).forEach((t) => pairs.push({ slug, target: t, ws, op: 'add' }));
    }
    const n = slugs.length;
    const noun = n === 1 ? slugs[0] : `${n} skills`;
    emit({ type: 'add', slugs, ws });
    return finishPlan({ kind: 'add', title: `Adding ${noun}`, toast: `Added ${noun}`, pairs });
  }

  function reset() {
    state = clone(INITIAL);
    emit({ type: 'reset' });
  }
  function on(fn) {
    listeners.add(fn);
    return () => listeners.delete(fn);
  }

  window.DATA = {
    targets, wsDirs, folders, skills, discoveryCount: 2,
    skill, target, wsDir, workspace, preset, activePreset,
    presets: () => state.presets, workspaces: () => state.workspaces,
    presetRaw, presetHolds, wsRaw, workspaceHolds, workspaceDirs,
    isActive, whereActive, agentSkills, workspaceSkills, targetsLabel, isEdited, placeName,
    filterSkills, counts, sidebarCounts,
    setAgents, removeAgent, setWorkspaceDirs, removeWorkspaceDir,
    planSwitch, switchPreset, deletePreset, addSkills, reset, on,
  };
})();
