// osascript -l JavaScript panel-pick.js <pid> <name> [<name>...]: in the app's Open panel (list
// view), expand each folder named in turn below the previous one and select the last. Pure
// accessibility calls: the app is never activated and no keystroke is sent.
function run(argv) {
  const se = Application('System Events');
  const p = se.processes.whose({ unixId: Number(argv[0]) })[0];
  const w = p.windows.byName('Open');
  const list = w.splitterGroups[0].splitterGroups[0].scrollAreas[0].outlines[0];
  const first = (r) => r.uiElements[0];
  const nameOf = (r) => {
    try { return first(r).textFields[0].value(); } catch (x) { return ''; }
  };
  let from = 0, row = null;
  const names = argv.slice(1);
  names.forEach((name, k) => {
    const rows = list.rows();
    let i = from;
    while (i < rows.length && nameOf(rows[i]) !== name) i++;
    if (i >= rows.length) throw new Error('row not found: ' + name);
    row = rows[i];
    if (k < names.length - 1) {
      const kids = first(row).uiElements();
      const tri = kids.find((e) => e.role() === 'AXDisclosureTriangle');
      if (tri.value() === 0) tri.click();
      delay(0.8);
    }
    from = i + 1;
  });
  row.selected = true;
  return 'selected ' + nameOf(row);
}
