// Usage: node render.mjs <pose.json> [out.png]
// Writes a 2x2 multi-view PNG and prints a JSON report (limits, penetrations, fingertip gaps).
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';

const root = path.dirname(fileURLToPath(import.meta.url));
const [poseFile, outArg] = process.argv.slice(2);
if (!poseFile) { console.error('usage: node render.mjs <pose.json> [out.png]'); process.exit(1); }
const pose = JSON.parse(fs.readFileSync(poseFile, 'utf8'));
const out = outArg || poseFile.replace(/\.json$/i, '') + '.png';

const types = { '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript' };
const server = http.createServer((req, res) => {
  const file = path.join(root, decodeURIComponent(req.url.split('?')[0]));
  if (!file.startsWith(root) || !fs.existsSync(file)) { res.writeHead(404); return res.end(); }
  res.writeHead(200, { 'content-type': types[path.extname(file)] || 'application/octet-stream' });
  fs.createReadStream(file).pipe(res);
}).listen(0);
const port = server.address().port;

function findChrome() {
  const base = path.join(process.env.LOCALAPPDATA || '', 'ms-playwright');
  const dirs = fs.readdirSync(base).filter((d) => /^chromium-\d+$/.test(d)).sort().reverse();
  for (const d of dirs) for (const sub of ['chrome-win64', 'chrome-win']) {
    const exe = path.join(base, d, sub, 'chrome.exe');
    if (fs.existsSync(exe)) return exe;
  }
  throw new Error('no Playwright chromium found under ' + base);
}

const browser = await chromium.launch({ executablePath: findChrome(), args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
try {
  const page = await browser.newPage({ viewport: { width: 1024, height: 1024 } });
  const errors = [];
  page.on('pageerror', (e) => errors.push(e.message));
  await page.goto(`http://localhost:${port}/render.html`);
  await page.waitForFunction(() => window.ready === true, null, { timeout: 15000 });
  const report = await page.evaluate((p) => window.renderPose(p), pose);
  await page.locator('#wrap').screenshot({ path: out });
  if (errors.length) report.page_errors = errors;
  report.image = path.resolve(out);
  // Keep number arrays on one line so the report stays short.
  console.log(JSON.stringify(report, null, 1).replace(/\[\s+([-\d.,\s]+?)\s+\]/g, (_, xs) => `[${xs.replace(/\s+/g, ' ')}]`));
} finally {
  await browser.close();
  server.close();
}
