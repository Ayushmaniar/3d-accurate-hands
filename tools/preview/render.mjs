// Usage: node render.mjs <geometry.json> <out.png> [views=front,side,top,hero] [size=384]
// Called by `handkit render`; draws `handkit geometry` output headlessly.
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';

const root = path.dirname(fileURLToPath(import.meta.url));
const [geomFile, out, viewArg = 'front,side,top,hero', sizeArg = '384'] = process.argv.slice(2);
if (!geomFile || !out) { console.error('usage: node render.mjs <geometry.json> <out.png> [views] [size]'); process.exit(1); }
const geom = JSON.parse(fs.readFileSync(geomFile, 'utf8'));

const types = { '.html': 'text/html', '.js': 'text/javascript' };
const server = http.createServer((req, res) => {
  const file = path.join(root, decodeURIComponent(req.url.split('?')[0]));
  if (!file.startsWith(root) || !fs.existsSync(file)) { res.writeHead(404); return res.end(); }
  res.writeHead(200, { 'content-type': types[path.extname(file)] || 'application/octet-stream' });
  fs.createReadStream(file).pipe(res);
}).listen(0);

function findChrome() {
  if (process.env.HANDKIT_CHROME) return process.env.HANDKIT_CHROME;
  const base = path.join(process.env.LOCALAPPDATA || path.join(process.env.HOME || '', '.cache'), 'ms-playwright');
  const dirs = fs.readdirSync(base).filter((d) => /^chromium-\d+$/.test(d)).sort().reverse();
  for (const d of dirs) for (const sub of ['chrome-win64/chrome.exe', 'chrome-win/chrome.exe', 'chrome-linux/chrome', 'chrome-mac/Chromium.app/Contents/MacOS/Chromium']) {
    const exe = path.join(base, d, sub);
    if (fs.existsSync(exe)) return exe;
  }
  throw new Error(`no Playwright Chromium under ${base}; set HANDKIT_CHROME`);
}

const browser = await chromium.launch({ executablePath: findChrome(), args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
try {
  const page = await browser.newPage({ viewport: { width: 1200, height: 1200 } });
  const errors = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto(`http://localhost:${server.address().port}/render.html`);
  await page.waitForFunction(() => window.ready === true, null, { timeout: 15000 });
  await page.evaluate(([g, v, s]) => window.render(g, v, s), [geom, viewArg.split(',').map((s) => s.trim()), Number(sizeArg)]);
  if (errors.length) throw new Error(errors.join('\n'));
  await page.locator('#wrap').screenshot({ path: out });
} finally {
  await browser.close();
  server.close();
}
