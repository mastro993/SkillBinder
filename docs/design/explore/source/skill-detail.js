/* Explore · skill detail panel (P1 with I1-I6): the panel's pieces, which each variant lays out.
 * Load after explore-kit.js. Styles (.sd-*) live in skill-detail.html.
 *
 * SD.parts(slug, {win, onShowLibrary, onCreator, fetchMd}) -> {s, title, install, desc, error, git,
 *     meta, filesLabel, files, mdLabel, md}. fetchMd(slug, attempt) -> Promise<string> (default
 *     CATALOG.skillMd); a rejection shows "Couldn't load SKILL.md" with Retry (attempt + 1).
 * SD.stack(parts, end?) -> the one-column panel: sticky header (name, Install, optional end
 *     control such as Close), then description, facts, files, and SKILL.md.
 * SD.fileTree(paths) collapsible folders; SD.markdown(text) -> Element (front matter dropped).
 */
(function () {
  'use strict';
  const C = window.CATALOG;
  const { h, ic, toast, spinner } = window.SB;
  const { live, installAction, installErrorLine, creatorAvatar, installsMeta, sourceTag } = window.EX;

  // ---- SKILL.md as text: headings, paragraphs, lists, code ------------------------------
  const esc = (t) => t.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  const inline = (t) => esc(t).replace(/`([^`]+)`/g, '<code>$1</code>').replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
  function markdown(src) {
    const lines = src.replace(/^---\n[\s\S]*?\n---\n+/, '').split('\n');
    const out = [];
    let para = [];
    let list = null;
    const flush = () => {
      if (para.length) out.push(`<p>${inline(para.join(' '))}</p>`);
      if (list) out.push(`<${list.tag}>${list.items.map((i) => `<li>${inline(i)}</li>`).join('')}</${list.tag}>`);
      para = [];
      list = null;
    };
    for (let i = 0; i < lines.length; i++) {
      const l = lines[i];
      if (l.startsWith('```')) {
        flush();
        const code = [];
        while (++i < lines.length && !lines[i].startsWith('```')) code.push(lines[i]);
        out.push(`<pre><code>${esc(code.join('\n'))}</code></pre>`);
        continue;
      }
      const hd = l.match(/^(#{1,3})\s+(.*)/);
      const li = l.match(/^\s*(?:[-*]|(\d+)\.)\s+(.*)/);
      if (hd) {
        flush();
        out.push(`<h${hd[1].length}>${inline(hd[2])}</h${hd[1].length}>`);
      } else if (li) {
        const tag = li[1] ? 'ol' : 'ul';
        if (para.length || (list && list.tag !== tag)) flush();
        if (!list) list = { tag, items: [] };
        list.items.push(li[2]);
      } else if (!l.trim()) {
        flush();
      } else {
        if (list) flush();
        para.push(l.trim());
      }
    }
    flush();
    return h('div', { class: 'sd-md', html: out.join('') });
  }

  // ---- files: folders collapse, folders first ---------------------------------------------
  function fileTree(paths) {
    const root = { dirs: new Map(), files: [] };
    paths.forEach((p) => {
      const segs = p.split('/');
      let n = root;
      segs.slice(0, -1).forEach((d) => {
        if (!n.dirs.has(d)) n.dirs.set(d, { dirs: new Map(), files: [] });
        n = n.dirs.get(d);
      });
      n.files.push(segs[segs.length - 1]);
    });
    const count = (n) => n.files.length + [...n.dirs.values()].reduce((t, d) => t + count(d), 0);
    const glyph = (f) => (/\.(md|txt)$/i.test(f) ? 'book' : 'code');
    const rows = (n, d) => [
      ...[...n.dirs].map(([name, dir]) => {
        const kids = h('div', { hidden: true }, rows(dir, d + 1));
        const row = h('button', { class: 'sd-tree-row', style: `--d:${d}`, 'aria-expanded': 'false', onclick: () => {
          kids.hidden = !kids.hidden;
          row.setAttribute('aria-expanded', String(!kids.hidden));
        } }, ic('chevron-right', 12, 'sd-chev'), ic('folder', 13), h('span', { class: 'sd-tree-name' }, name),
        h('span', { class: 'sb-chip-count' }, count(dir)));
        return [row, kids];
      }),
      ...n.files.map((f) => h('div', { class: 'sd-tree-row', style: `--d:${d}` },
        h('span', { class: 'sd-chev-sp' }), ic(glyph(f), 13), h('span', { class: 'sd-tree-name' }, f))),
    ];
    return h('div', { class: 'sd-tree' }, rows(root, 0));
  }

  // ---- SKILL.md: skeleton while fetched, one line + Retry when it fails ------------------
  function mdBlock(slug, fetchMd) {
    const el = h('div', { class: 'sd-mdbox' });
    let attempt = 0;
    const skeleton = () => h('div', { class: 'sd-skel', role: 'status', 'aria-label': 'Loading SKILL.md' },
      [46, 92, 84, 64, 0, 38, 88, 72].map((w) => (w ? h('i', { style: `width:${w}%` }) : h('b'))));
    function load() {
      el.replaceChildren(skeleton());
      fetchMd(slug, attempt++).then((t) => el.replaceChildren(markdown(t)), () => el.replaceChildren(
        h('div', { class: 'ex-error sd-md-error', role: 'status' }, ic('alert-circle', 13), "Couldn't load SKILL.md",
          h('button', { class: 'sb-btn sb-btn-outline sb-btn-xs', onclick: load }, 'Retry'))));
    }
    load();
    return el;
  }

  // ---- I5: without Git, the app's Install Git action sits under the disabled Install ------
  // One Git install for the page: it survives the panel being redrawn; a fixture reset drops it.
  let gitRun = 0;
  let gitBusy = false;
  C.on((e) => { if (e.type === 'reset') { gitRun++; gitBusy = false; } });
  function gitLine(slug, win) {
    const draw = (el) => {
      const show = C.installState(slug) === 'needs-git';
      el.hidden = !show;
      if (!show) { el.replaceChildren(); return; }
      const btn = gitBusy ? h('button', { class: 'sb-btn sb-btn-outline sb-btn-xs', disabled: true }, spinner(12), 'Installing Git')
        : h('button', { class: 'sb-btn sb-btn-outline sb-btn-xs', onclick: () => {
          const run = gitRun;
          gitBusy = true;
          draw(el);
          setTimeout(() => {
            if (run !== gitRun) return;
            gitBusy = false;
            C.setSetting('git', true);
            if (win) toast(win, 'Git installed', { icon: 'check-circle' });
          }, 1500);
        } }, 'Install Git');
      el.replaceChildren(ic('git-branch', 13), h('span', 'Installing needs Git'), btn);
    };
    return live(h('div', { class: 'sd-git' }), draw);
  }

  function repoLink(s, win) {
    return h('button', { class: 'ex-link', 'data-tip': 'Open in browser · new icon',
      onclick: () => win && toast(win, `Opens ${s.url.replace(/^https?:\/\//, '')}`) },
    ic(s.private ? 'lock' : 'github', 14), h('span', { class: 'sb-ellipsis' }, s.repo), h.html(window.newIcon('link', 12)));
  }
  const label = (text, n) => h('div', { class: 'ex-detail-label' }, text, n == null ? null : h('span', { class: 'sb-chip-count' }, n));

  function parts(slug, { win, onShowLibrary, onCreator, fetchMd = C.skillMd } = {}) {
    const s = C.skill(slug);
    // A repository's own link already names it; a creator or skills.sh is worth its own mark.
    const by = s.origin === 'catalog'
      ? h('button', { class: 'ex-link', onclick: () => onCreator && onCreator(s.creator) }, creatorAvatar(s.creator, { size: 16 }), s.creator)
      : s.origin === 'skills.sh' ? sourceTag(s) : null;
    return {
      s,
      title: h('div', { class: 'sd-title' }, s.name),
      install: installAction(slug, { win, onShowLibrary }),
      desc: h('div', { class: 'sd-desc' }, s.desc),
      error: installErrorLine(slug),
      git: gitLine(slug, win),
      meta: h('div', { class: 'sd-meta' }, by, repoLink(s, win), installsMeta(s)),
      filesLabel: label('Files', s.files.length),
      files: fileTree(s.files),
      mdLabel: label('SKILL.md'),
      md: mdBlock(slug, fetchMd),
    };
  }
  const stack = (p, end) => h('div', { class: 'sd-panel' },
    h('div', { class: 'sd-head' }, p.title, p.install, end || null),
    h('div', { class: 'sd-body' }, p.desc, p.error, p.git, p.meta, p.filesLabel, p.files, p.mdLabel, p.md));

  window.SD = { parts, stack, fileTree, markdown };
})();
