/* The mock window: its icons, the palette resolved into CSS variables per accent role, the
   sidebar, and the Library, workspace, Explore, and Import dialog views. */
const S = (d, extra = '') => `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" ${extra}>${d}</svg>`;
const ICON = {
  explore: S('<circle cx="12" cy="12" r="9"/><path d="M15.5 8.5l-2 5-5 2 2-5z"/>'),
  library: S('<path d="M5 4v16M9 4v16M13 5l4 15M19 4v16"/>'),
  workspace: S('<path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2z"/>'),
  agent: S('<rect x="3" y="4" width="18" height="16" rx="3"/><path d="M7 10l3 2-3 2M12 15h4"/>'),
  settings: S('<circle cx="12" cy="12" r="3"/><path d="M12 3v2.5M12 18.5V21M3 12h2.5M18.5 12H21M5.6 5.6l1.8 1.8M16.6 16.6l1.8 1.8M5.6 18.4l1.8-1.8M16.6 7.4l1.8-1.8"/>'),
  stack: S('<path d="M12 3l9 5-9 5-9-5z"/><path d="M3 13l9 5 9-5"/>'),
  code: S('<path d="M8 8l-4 4 4 4M16 8l4 4-4 4"/>'),
  pen: S('<path d="M4 20h4L19 9l-4-4L4 16z"/>'),
  panel: S('<rect x="3" y="4" width="18" height="16" rx="3"/><path d="M9 4v16"/>'),
  left: S('<path d="M15 6l-6 6 6 6"/>'), right: S('<path d="M9 6l6 6-6 6"/>'),
  plus: S('<path d="M12 5v14M5 12h14"/>'), search: S('<circle cx="11" cy="11" r="6"/><path d="M20 20l-4.5-4.5"/>'),
  check: S('<path d="M5 12.5l4.5 4.5L19 7.5"/>', 'stroke-width="2.6"'),
  up: S('<circle cx="12" cy="12" r="9"/><path d="M12 16V8M8.5 11.5L12 8l3.5 3.5"/>'),
  scan: S('<path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2z"/><circle cx="12" cy="13" r="2.5"/>'),
  warn: S('<path d="M12 4l9 16H3z"/><path d="M12 10v4M12 17v.5"/>'),
  claude: '<svg viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="5" fill="#D97757"/><path d="M12 6v12M6 12h12M7.8 7.8l8.4 8.4M16.2 7.8l-8.4 8.4" stroke="#FFF" stroke-width="2" stroke-linecap="round"/></svg>',
  codex: '<svg viewBox="0 0 24 24"><rect x="2" y="2" width="20" height="20" rx="5" fill="var(--fg)"/><path d="M8 9l3 3-3 3M13 15h3" stroke="var(--bg)" stroke-width="2" fill="none" stroke-linecap="round"/></svg>',
};

function resolve(t, look) {
  const r = look.roles, light = state.mode === 'light';
  const graphite = t.foreground, graphiteInk = t.background;
  const v = {
    '--bg': t.background, '--fg': t.foreground, '--sidebar': t.sidebar, '--muted': t.muted, '--muted-fg': t['muted-foreground'],
    '--border': t.border, '--hairline': t.hairline, '--popover': t.popover, '--icon': t['sidebar-foreground'],
    '--label': { muted: t['muted-foreground'], sidebar: t['sidebar-foreground'], fg: t.foreground }[look.labels],
    '--section': t['muted-foreground'],
    '--btn-bg': r.primaryButton ? t.primary : graphite, '--btn-fg': r.primaryButton ? t['primary-foreground'] : graphiteInk,
    '--focus': r.focus ? t.ring : mix(t['muted-foreground'], t.background, 0.35),
    '--check': r.checks ? t.primary : graphite, '--check-fg': r.checks ? t['primary-foreground'] : graphiteInk,
    '--switch': r.checks ? t.primary : graphite, '--switch-off': light ? mix(t.border, t.foreground, 0.12) : t.border,
    '--progress': r.progress ? t.primary : graphite,
    '--link': r.links ? (light ? t['accent-soft-foreground'] : t['accent-soft-foreground']) : t.foreground, '--link-deco': r.links ? 'none' : 'underline',
    '--chip-bg': r.chips ? t['accent-soft'] : t.muted, '--chip-fg': r.chips ? t['accent-soft-foreground'] : t.foreground,
    '--tab-fg': r.tabs ? t['accent-soft-foreground'] : t.foreground,
    '--tab-bg': light ? t.background : t['sidebar-accent'], '--tab-shadow': '0 0 0 .5px rgba(0,0,0,.08), 0 1px 2px rgba(0,0,0,.08)',
    '--sel-fg': t.foreground,
  };
  return v;
}

function glyph(t, look, id, selected, svg = ICON[id], navKey = id) {
  const color = t[`nav-${navKey}`];
  if (look.icons === 'tile') return `<span class="g tile" style="background:${color};color:#FFF">${svg}</span>`;
  if (look.icons === 'glyph') return `<span class="g" style="color:${color}">${svg}</span>`;
  const mono = selected ? (look.roles.selGlyph ? t.primary : t.foreground) : t['muted-foreground'];
  return `<span class="g" style="color:${mono}">${svg}</span>`;
}

function selStyle(t, look, color) {
  const light = state.mode === 'light';
  const raised = light ? '0 0 0 .5px rgba(0,0,0,.08), 0 1px 2px rgba(0,0,0,.08)' : '0 1px 2px rgba(0,0,0,.4), 0 0 0 1px rgba(255,255,255,.04)';
  switch (look.sel) {
    case 'pill': return `background:${t['sidebar-accent']};box-shadow:${raised}`;
    case 'fill': return `background:${t['sidebar-accent']}`;
    case 'accent': return `background:${t['accent-soft']};color:${t['accent-soft-foreground']}`;
    case 'item': return `background:${mix(t.sidebar, color, light ? 0.16 : 0.26)}`;
    case 'bar': return `background:${t['sidebar-accent']};box-shadow:inset 3px 0 0 ${t.primary}, ${raised}`;
  }
}

function sidebar(t, look) {
  const page = state.page;
  const row = (key, label, n, selected, g, color, extra = '') =>
    `<div class="srow ${selected ? 'sel' : ''}" style="${selected ? selStyle(t, look, color) : ''}">${g}<span>${label}</span>${extra}<span class="n">${n ?? ''}</span></div>`;
  const nav = (id, label, n, sel) => row(id, label, n, sel, glyph(t, look, id, sel), t[`nav-${id}`]);
  const P = PRESET[state.mode];
  const preset = (label, n, color, icon, active) => {
    const g = look.icons === 'tile' ? `<span class="g tile" style="background:${color};color:#FFF">${ICON[icon]}</span>` : `<span class="g" style="color:${color}">${ICON[icon]}</span>`;
    return row('', label, n, false, g, color, active ? `<span class="check">${ICON.check}</span>` : '');
  };
  const ws = (label, n, sel) => row('', label, n, sel, glyph(t, look, 'workspace', sel), t['nav-workspace']);
  const agentLogo = (svg) => `<span class="g">${svg}</span>`;
  return `<div class="side"><div class="ctrls"><span class="ib">${ICON.panel}</span><span class="sp"></span><span class="ib">${ICON.left}</span><span class="ib off">${ICON.right}</span></div>
    <div class="rows">
      ${nav('explore', 'Explore', null, page === 'explore')}
      ${nav('library', 'Library', 9, page === 'library')}
      <div class="sec">Presets</div>
      ${preset('Default', 9, P.blue, 'stack', true)}${preset('Frontend', 4, P.orange, 'code')}${preset('Writing', 3, P.pink, 'pen')}
      <div class="sec">Workspaces</div>
      ${ws('atlas-web', 4, page === 'workspace')}${ws('orbit-api', 2, false)}
      <div class="sec">Agents</div>
      ${row('', 'Universal', 5, false, glyph(t, look, 'agent', false), t['nav-agent'])}
      ${row('', 'Claude Code', 7, false, agentLogo(ICON.claude), '#D97757')}
      ${row('', 'Codex', 3, false, agentLogo(ICON.codex), t.foreground)}
      <div class="spacer"></div>
      ${nav('settings', 'Settings', null, false)}
    </div></div>`;
}

function libraryPage(t) {
  const P = PRESET[state.mode];
  const rows = [
    ['api-design', 'Designs HTTP APIs the Acme way: resource names, pagination, errors.', `<span class="chip hl">New</span><span class="chip out"><i style="background:${P.blue}"></i>Default</span>`, true, ''],
    ['changelog-writer', 'Keeps CHANGELOG.md in the Keep a Changelog format.', `<span class="status" style="color:${t['info-text']}"><i style="background:${t.info}"></i>Update</span><span class="chip out"><i style="background:${P.orange}"></i>Frontend</span>`, true, 'hover'],
    ['commit-messages', 'Writes Conventional Commits messages from a staged diff.', `<span class="chip out"><i style="background:${P.pink}"></i>Writing</span>`, true, ''],
    ['incident-runbook', 'Walks through the first thirty minutes of an incident.', `<span class="chip">Not in a preset</span>`, false, 'inactive'],
  ];
  return `<div class="head"><div class="title"><span class="tab sel">All skills</span><span class="tab">Work</span><span class="tab">Personal</span><span class="vsep"></span><span class="ib" style="color:var(--muted-fg)">${ICON.plus}</span></div>
    <div class="actions"><span class="chip hl" style="height:24px">${ICON.up.replace('<svg', '<svg width="12" height="12"')}Updates 2</span><span class="ib" style="color:var(--muted-fg)">${ICON.scan}</span></div></div>
    <div class="content"><div class="toolbar"><div class="search focus">${ICON.search}api<span class="caret"></span></div><div class="segc"><span class="on">All</span><span>Active</span><span>Inactive</span></div></div>
    <div class="card">${rows.map(([n, d, marks, on, cls]) => `<div class="item ${cls}"><div class="txt"><b>${n}</b><span class="d">${d}</span></div>${marks}<span class="switch sw-ctl ${on ? 'on' : ''}"></span></div>`).join('')}</div>
    <div class="notice" style="color:${t['warning-text']}">${ICON.warn}2 copies changed outside SkillBinder. <span class="link">Review</span></div></div>
    <div class="pill">Syncing 3 of 7<span class="bar"><i></i></span></div>`;
}

function workspacePage(t, look) {
  const rows = [['release-notes', 'Drafts user-facing release notes from merged pull requests.', true], ['api-design', 'Designs HTTP APIs the Acme way.', true], ['commit-messages', 'Writes Conventional Commits messages from a staged diff.', false]];
  return `<div class="head"><div class="title">${glyph(t, look, 'workspace', true)}<span class="name">atlas-web</span><span style="color:var(--muted-fg)">·</span><span class="path mono">~/Projects/clients/acme/atlas/apps/web</span></div>
    <div class="actions"><span class="btn primary focus">${ICON.plus}Add skill</span></div></div>
    <div class="content"><div class="sub">AGENTS</div><div style="display:flex;gap:6px"><span class="chip out" style="height:24px">${ICON.claude.replace('<svg', '<svg width="14" height="14"')}Claude Code</span><span class="chip out" style="height:24px">${ICON.codex.replace('<svg', '<svg width="14" height="14"')}Codex</span><span class="chip" style="height:24px;background:var(--bg);box-shadow:inset 0 0 0 1px var(--border);border:1px dashed transparent">+ Agent</span></div>
    <div class="card">${rows.map(([n, d, on]) => `<div class="item"><span class="cb ${on ? 'on' : ''}">${on ? ICON.check : ''}</span><div class="txt"><b>${n}</b><span class="d">${d}</span></div><span class="status" style="color:${t['success-text']}"><i style="background:${t.success}"></i>Copied</span></div>`).join('')}</div>
    <div class="notice" style="color:${t['info-text']}">${ICON.up}1 update from its source. <span class="link">Update all</span></div>
    <div class="notice" style="color:${t['danger-text']}">${ICON.warn}Could not write to ~/.codex/skills.</div></div>`;
}

function explorePage(t) {
  const cards = [['pdf-tools', 'Fill, merge and split PDFs.', 'anthropics/skills', 'new'], ['frontend-design', 'Distinctive, intentional UI.', 'anthropics/skills', 'installed'], ['tdd', 'Red, green, refactor.', 'mattpocock/skills', ''], ['changelog', 'Keep a Changelog entries.', 'acme/skills', ''], ['security-review', 'Review pending changes.', 'acme/skills', 'new'], ['diagrams', 'Mermaid and SVG diagrams.', 'jdoe/skills', '']];
  return `<div class="head"><div class="title"><span class="name">Explore</span></div><div class="actions"><div class="segc"><span class="on">Catalog</span><span>skills.sh</span><span>Repositories</span></div></div></div>
    <div class="content"><div class="toolbar"><div class="search">${ICON.search}Search skills</div></div>
    <div class="grid">${cards.map(([n, d, o, s]) => `<div class="card tile-card"><div class="top"><b>${n}</b>${s === 'new' ? '<span class="chip hl">New</span>' : ''}</div><span class="d" style="color:var(--muted-fg)">${d}</span>
      <div class="foot"><span class="mono" style="color:var(--muted-fg);font-size:11px">${o}</span>${s === 'installed' ? `<span class="status" style="color:${t['success-text']}">${ICON.check.replace('<svg', '<svg width="12" height="12"')}Installed</span>` : '<span class="btn sm">Install</span>'}</div></div>`).join('')}</div>
    <div style="font-size:12px;color:var(--muted-fg)">Browsing the catalog. <span class="link">Add a repository</span></div></div>`;
}

function dialog() {
  const rows = [['api-design', true], ['changelog-writer', true], ['commit-messages', true], ['incident-runbook', false]];
  return `<div class="scrim"><div class="dialog"><h3>Import skills</h3><div style="font-size:13px;color:var(--muted-fg)">Found 4 skills in ~/.claude/skills.</div>
    <div>${rows.map(([n, on]) => `<div class="lrow"><span class="cb ${on ? 'on' : ''}">${on ? ICON.check : ''}</span><span style="font-weight:600;font-size:14px">${n}</span></div>`).join('')}</div>
    <div class="foot"><span class="btn">Cancel</span><span class="btn primary">Import 3 skills</span></div></div></div>`;
}

function renderWin() {
  const look = comparing ? lookOf('cur') : state.look;
  const t = generate(look, state.mode);
  const win = document.getElementById('win');
  win.setAttribute('style', Object.entries(resolve(t, look)).map(([k, v]) => `${k}:${v}`).join(';'));
  const page = state.page === 'library' ? libraryPage(t) : state.page === 'workspace' ? workspacePage(t, look) : explorePage(t);
  win.innerHTML = sidebar(t, look) + `<div class="main">${page}</div>` + `<div class="lights"><i></i><i></i><i></i></div>` + (state.dialog ? dialog() : '');
  fit();
  return t;
}

function fit() {
  const stage = document.getElementById('stage'), win = document.getElementById('win');
  const s = Math.min(1, stage.clientWidth / 1180);
  win.style.transform = `scale(${s})`;
  stage.style.height = `${660 * s}px`;
}
