// node verify.js <prototype.html> <root>: compare the prototypes' fixture (kit/data.js, inlined in
// every bundle under ../prototypes) with the native seed (library SKILL.md frontmatter + the
// app database read through the sqlite3 CLI). Exit 1 on drift.
const fs = require('fs'), path = require('path'), { execFileSync } = require('child_process');
const [bundle, root] = process.argv.slice(2);
const inlined = fs.readFileSync(bundle, 'utf8').match(/<script>\/\* kit\/data\.js \*\/\n([\s\S]*?)\n<\/script>/);
if (!inlined) throw new Error(`${bundle} holds no inlined kit/data.js`);
const window = {}; new Function('window', inlined[1])(window);
const D = window.DATA, db = path.join(root, 'data/skillbinder.db');
const q = (sql) => execFileSync('sqlite3', ['-readonly', '-separator', '\t', db, sql], { encoding: 'utf8' })
  .trim().split('\n').filter(Boolean).map((l) => l.split('\t'));
const problems = [];
const check = (ok, msg) => { if (!ok) problems.push(msg); };
const folders = Object.fromEntries(q("select s.slug, coalesce(f.name,'') from skills s left join library_folders f on f.id = s.folder_id"));
check(Object.keys(folders).length === D.skills.length, `skill count ${Object.keys(folders).length} != ${D.skills.length}`);
for (const s of D.skills) {
  const md = fs.readFileSync(path.join(root, 'data/library/skills', s.slug, 'SKILL.md'), 'utf8');
  const desc = (md.match(/^description: (.*)$/m) || [])[1];
  check(desc === s.desc, `${s.slug}: description "${desc}"`);
  check((folders[s.slug] || null) === s.folder, `${s.slug}: folder "${folders[s.slug]}" != "${s.folder}"`);
}
const ALL_UNIVERSAL = 'cline,codex,dexto,fx,kimi-code-cli,loaf,pi,sarvam-code,warp,zed';
const native = { claude: 'claude-code', cursor: 'cursor', windsurf: 'windsurf', universal: ALL_UNIVERSAL };
const want = (raw) => raw === 'all' ? '' : raw.map((t) => native[t]).join(',').split(',').sort().join(',');
const presetRows = q("select p.name, s.slug, coalesce(s.agents,'') from preset_skills s join presets p on p.id = s.preset_id");
for (const p of D.presets()) {
  const got = Object.fromEntries(presetRows.filter((r) => r[0] === p.name).map((r) => [r[1], JSON.parse(r[2] || '[]').sort().join(',')]));
  check(Object.keys(got).length === Object.keys(p.skills).length, `${p.name}: ${Object.keys(got).length} skills`);
  for (const [slug, raw] of Object.entries(p.skills)) check(got[slug] === want(raw), `${p.name}/${slug}: "${got[slug]}" != "${want(raw)}"`);
}
const dirOf = { claude: '.claude/skills', agents: '.agents/skills', windsurf: '.windsurf/skills' };
const wsRows = q("select w.name, s.slug, coalesce(s.dirs,'') from workspace_skills s join workspaces w on w.id = s.workspace_id");
const wsDirs = q("select w.name, d.dir from workspace_dirs d join workspaces w on w.id = d.workspace_id");
for (const w of D.workspaces()) {
  const got = Object.fromEntries(wsRows.filter((r) => r[0] === w.name).map((r) => [r[1], JSON.parse(r[2] || '[]').sort().join(',')]));
  check(Object.keys(got).length === Object.keys(w.skills).length, `${w.name}: ${Object.keys(got).length} skills`);
  for (const [slug, raw] of Object.entries(w.skills)) {
    const exp = raw === 'all' ? '' : raw.map((d) => dirOf[d]).sort().join(',');
    check(got[slug] === exp, `${w.name}/${slug}: "${got[slug]}" != "${exp}"`);
  }
  const dirs = wsDirs.filter((r) => r[0] === w.name).map((r) => r[1]).sort().join(',');
  check(dirs === w.dirs.map((d) => dirOf[d]).sort().join(','), `${w.name}: dirs "${dirs}"`);
}
const colors = Object.fromEntries(q("select name, coalesce(color,'') from presets"));
check(colors.Frontend === '#3b82f6' && colors.Backend === '#f59e0b' && colors.Default === '', `colors ${JSON.stringify(colors)}`);
const active = q("select value from device_settings where key = 'active_preset'")[0][0];
check(active === '"default"', `active preset ${active}`);
const sourced = q("select slug from skills where coalesce(source,'') != '' order by slug").map((r) => r[0]).join(',');
check(sourced === D.skills.filter((s) => s.hasUpdate).map((s) => s.slug).join(','), `sources on ${sourced}`);
console.log(problems.length ? problems.join('\n') : `OK: ${D.skills.length} skills, presets, workspaces, folders, colors and sources match data.js`);
process.exit(problems.length ? 1 : 0);
