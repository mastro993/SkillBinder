// Drive headless Chrome over the DevTools pipe (no dependencies): open a reference-mode URL at
// an exact viewport and scale, wait for the page's <pre id="__ref_result">, save a PNG of the
// viewport and print the result JSON on stdout. One run serves both, so the spec and the PNG
// come from the same layout (Chrome's --dump-dom and --screenshot get different viewports).
//
// usage: node ref-chrome.mjs <url> <out.png> <width> <height> <scale> <light|dark> [timeoutMs]
import { spawn } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const [url, out, w, h, scale, theme, timeout = '20000'] = process.argv.slice(2);
if (!theme) {
  console.error('usage: node ref-chrome.mjs <url> <out.png> <width> <height> <scale> <light|dark> [timeoutMs]');
  process.exit(2);
}
const CHROME = process.env.CHROME || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const profile = mkdtempSync(join(process.env.TMPDIR || tmpdir(), 'sbref-'));
const chrome = spawn(CHROME, [
  '--headless=new', '--disable-gpu', '--no-first-run', '--no-default-browser-check', '--disable-extensions',
  '--disable-background-networking', '--disable-component-update', '--disable-sync', '--mute-audio',
  '--hide-scrollbars', '--force-color-profile=srgb', `--user-data-dir=${profile}`, `--window-size=${w},${h}`,
  `--force-device-scale-factor=${scale}`, '--remote-debugging-pipe', 'about:blank',
], { stdio: ['ignore', 'ignore', 'ignore', 'pipe', 'pipe'] });

let pending = Buffer.alloc(0);
let seq = 0;
const waiting = new Map();
chrome.stdio[4].on('data', (chunk) => {
  pending = Buffer.concat([pending, chunk]);
  for (let i = pending.indexOf(0); i >= 0; i = pending.indexOf(0)) {
    const msg = JSON.parse(pending.subarray(0, i).toString('utf8'));
    pending = pending.subarray(i + 1);
    const call = msg.id && waiting.get(msg.id);
    if (!call) continue;
    waiting.delete(msg.id);
    if (msg.error) call.reject(new Error(`${call.method}: ${msg.error.message}`));
    else call.resolve(msg.result);
  }
});
function send(method, params = {}, sessionId) {
  return new Promise((resolve, reject) => {
    seq += 1;
    waiting.set(seq, { method, resolve, reject });
    chrome.stdio[3].write(`${JSON.stringify({ id: seq, method, params, ...(sessionId ? { sessionId } : {}) })}\0`);
  });
}
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function main() {
  const deadline = Date.now() + Number(timeout);
  let page = null;
  while (!page) {
    const { targetInfos } = await send('Target.getTargets');
    page = targetInfos.find((t) => t.type === 'page');
    if (!page) {
      if (Date.now() > deadline) throw new Error('Chrome opened no page');
      await sleep(20);
    }
  }
  const { sessionId } = await send('Target.attachToTarget', { targetId: page.targetId, flatten: true });
  const s = (method, params) => send(method, params, sessionId);
  await s('Emulation.setDeviceMetricsOverride', { width: Number(w), height: Number(h), deviceScaleFactor: Number(scale), mobile: false });
  await s('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: theme }] });
  await s('Page.enable');
  await s('Page.navigate', { url });
  const probe = "(e => e && location.href !== 'about:blank' ? e.textContent : null)(document.getElementById('__ref_result'))";
  for (;;) {
    const r = await s('Runtime.evaluate', { expression: probe, returnByValue: true }).catch(() => null);
    const text = r && r.result && r.result.value;
    if (text) {
      const shot = await s('Page.captureScreenshot', { format: 'png' });
      writeFileSync(out, Buffer.from(shot.data, 'base64'));
      process.stdout.write(`${text}\n`);
      return;
    }
    if (Date.now() > deadline) throw new Error(`the page never reported within ${timeout} ms`);
    await sleep(25);
  }
}

chrome.on('error', (e) => { console.error(`ref-chrome: cannot start Chrome: ${e.message}`); process.exit(1); });
main().then(() => 0, (e) => { console.error(`ref-chrome: ${e.message}`); return 1; }).then((code) => {
  const done = () => { rmSync(profile, { recursive: true, force: true }); process.exit(code); };
  if (chrome.exitCode !== null || chrome.signalCode !== null) done();
  else { chrome.once('exit', done); chrome.kill(); }
});
