/* Onboarding prototypes: shared fixture and helpers (after the kit's icons.js, data.js, shell.js).
 *
 * window.OB
 *   AGENTS   [{id, name, path, logo, members?}] found agents in the Agents section's order
 *            (Universal first, then by name); NEW = agents a later scan finds (notice, N1).
 *   GIT      {ok: '2.47.1', old: '2.30.1'}; CMD: Linux package commands by manager.
 *   agentRow(agent, control) -> row Element: logo, name, folder (Universal names its agents).
 *   checkboxRow(agent, on, onChange)   switchRow(agent, on, onChange, {note})
 *   sw(on, onChange, label) -> Kit Switch stand-in.   cmdBox(cmd, win) -> command box with Copy.
 *   btn(label, onclick, kind = 'outline'|'primary'|'ghost', disabled)
 *   foundLine(names) -> 'Found Codex.' | 'Found Codex and Windsurf.' | 'Found A, B, and 2 more.'
 *   waitFor(ms, fn) -> cancel()   a fake Git check that resolves after ms.
 */
(function () {
  'use strict';
  const T = DATA.targets;
  const AGENTS = [
    T.find((t) => t.id === 'universal'),
    T.find((t) => t.id === 'claude'),
    T.find((t) => t.id === 'cursor'),
    { id: 'gemini', name: 'Gemini CLI', path: '~/.gemini/skills', logo: 'gemini-cli' },
    { id: 'copilot', name: 'GitHub Copilot', path: '~/.copilot/skills', logo: 'github-copilot' },
  ];
  const NEW = [
    T.find((t) => t.id === 'windsurf'),
    { id: 'kiro', name: 'Kiro CLI', path: '~/.kiro/skills', logo: 'kiro-cli' },
    { id: 'hermes', name: 'Hermes Agent', path: '~/.hermes/skills', logo: 'square-terminal' },
  ];
  const GIT = { ok: '2.47.1', old: '2.30.1' };
  const CMD = { apt: 'sudo apt install git', dnf: 'sudo dnf install git', pacman: 'sudo pacman -S git' };

  function btn(label, onclick, kind, disabled) {
    const cls = { primary: 'sb-btn-primary', ghost: '', outline: 'sb-btn-outline' }[kind || 'outline'];
    return h('button', { class: `sb-btn ${cls}`, onclick, disabled }, label);
  }

  const logo = (a) => avatar(a, { size: 24, tip: false });

  function agentRow(a, control, note) {
    const sub = a.members
      ? h('span', { class: 'ob-agent-sub', 'data-tip': a.members.join(', ') }, a.members.join(', '))
      : h('span', { class: 'ob-agent-sub' }, h('code', {}, a.path));
    return h('label', { class: 'ob-agent' }, logo(a),
      h('span', { class: 'ob-agent-text' }, h('span', { class: 'ob-agent-name' }, a.name), note || sub), control);
  }

  const checkboxRow = (a, on, onChange) => agentRow(a, checkbox({ checked: on, onChange }));

  function sw(on, onChange, label) {
    const el = h('button', { class: 'ob-switch', role: 'switch', 'aria-checked': String(!!on), 'aria-label': label,
      onclick: (e) => { e.preventDefault(); const v = el.getAttribute('aria-checked') !== 'true'; el.setAttribute('aria-checked', String(v)); onChange && onChange(v); } });
    return el;
  }

  const switchRow = (a, on, onChange, opts = {}) => agentRow(a, sw(on, onChange, a.name), opts.note);

  function cmdBox(cmd, win) {
    return h('div', { class: 'ob-cmd' }, h('code', {}, cmd),
      h('button', { class: 'sb-btn sb-btn-sm sb-btn-outline', onclick: () => toast(win, 'Copied') }, 'Copy'));
  }

  function foundLine(names) {
    if (names.length === 1) return `Found ${names[0]}.`;
    if (names.length === 2) return `Found ${names[0]} and ${names[1]}.`;
    return `Found ${names[0]}, ${names[1]}, and ${names.length - 2} more.`;
  }

  function waitFor(ms, fn) {
    const t = setTimeout(fn, ms);
    return () => clearTimeout(t);
  }

  window.OB = { AGENTS, NEW, GIT, CMD, btn, logo, agentRow, checkboxRow, sw, switchRow, cmdBox, foundLine, waitFor };
})();
