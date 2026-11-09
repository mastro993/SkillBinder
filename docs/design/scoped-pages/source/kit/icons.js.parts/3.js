
window.NEW_ICONS = {
  filter: '<svg xmlns="http://www.w3.org/2000/svg" aria-hidden="true" viewBox="0 0 24 24" fill="none"><path d="M4 5h16l-6 7.5V19l-4 1.5v-8L4 5Z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/></svg>',
  more: '<svg xmlns="http://www.w3.org/2000/svg" aria-hidden="true" viewBox="0 0 24 24" fill="none"><circle cx="5.5" cy="12" r="1.5" fill="currentColor"/><circle cx="12" cy="12" r="1.5" fill="currentColor"/><circle cx="18.5" cy="12" r="1.5" fill="currentColor"/></svg>',
  minus: '<svg xmlns="http://www.w3.org/2000/svg" aria-hidden="true" viewBox="0 0 24 24" fill="none"><path d="M5 12h14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>',
  undo: '<svg xmlns="http://www.w3.org/2000/svg" aria-hidden="true" viewBox="0 0 24 24" fill="none"><path d="M4 9h11a5 5 0 0 1 0 10h-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/><path d="M8 5 4 9l4 4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>',
  link: '<svg xmlns="http://www.w3.org/2000/svg" aria-hidden="true" viewBox="0 0 24 24" fill="none"><path d="M10 14a4 4 0 0 0 5.66 0l3-3a4 4 0 0 0-5.66-5.66l-1 1M14 10a4 4 0 0 0-5.66 0l-3 3a4 4 0 0 0 5.66 5.66l1-1" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>',
};

(function () {
  const COLOR_LOGOS = new Set(['claude-code', 'codex', 'kimi-code-cli']);
  let uid = 0;

  function sized(svg, size, cls) {
    let out = svg.replace('<svg ', `<svg width="${size}" height="${size}" class="sb-ic ${cls}" `);
    // Gradient ids must stay unique when a logo appears many times on a page.
    if (out.includes(' id="')) {
      const n = ++uid;
      out = out.replace(/id="([^"]+)"/g, `id="$1-${n}"`).replace(/url\(#([^)]+)\)/g, `url(#$1-${n})`);
    }
    return out;
  }

  window.isColorLogo = (name) => COLOR_LOGOS.has(name);

  window.icon = function icon(name, size = 16, cls = '') {
    const svg = window.ICONS[name];
    if (!svg) {
      console.error(`icon(): unknown icon "${name}"`);
      return `<span class="sb-ic-missing" style="width:${size}px;height:${size}px" title="missing icon ${name}">?</span>`;
    }
    return sized(svg, size, cls);
  };

  window.newIcon = function newIcon(name, size = 16, cls = '') {
    const svg = window.NEW_ICONS[name];
    if (!svg) {
      console.error(`newIcon(): unknown icon "${name}"`);
      return '';
    }
    return `<span class="sb-newicon" data-tip="new icon">${sized(svg, size, cls)}</span>`;
  };
})();
