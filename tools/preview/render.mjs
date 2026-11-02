// Usage: node render.mjs <geometry.json> <out.png> [views=front,side,top,hero] [size=384]
// Called by `handkit render`; draws `handkit geometry` output headlessly.
// A clip (`{"frames": [...], "size": [w, h]}` from `handkit sheet` / `handkit preview`) draws a
// contact sheet into <out.png>, or with `"video": true` one PNG per frame into the folder <out>.
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
const assets = geom.mesh?.files || {};
const server = http.createServer((req, res) => {
  const url = decodeURIComponent(req.url.split('?')[0]);
  const asset = url.match(/^\/asset\/(right|left)\.glb$/);
  if (asset && assets[asset[1]]) {
    res.writeHead(200, { 'content-type': 'model/gltf-binary' });
    return fs.createReadStream(assets[asset[1]]).pipe(res);
  }
  if (url === '/handkit-three.js') {
    res.writeHead(200, { 'content-type': 'text/javascript' });
    return fs.createReadStream(path.join(root, '../../packages/handkit-three/index.js')).pipe(res);
  }
  const file = path.join(root, url);
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
  await page.waitForFunction(() => window.ready === true, null, { timeout: 30000 });
  if (geom.frames) {
    await page.evaluate((spec) => window.setupFrames(spec), geom);
    const png = (url) => Buffer.from(url.slice(url.indexOf(',') + 1), 'base64');
    if (geom.video) {
      fs.mkdirSync(out, { recursive: true });
      for (let i = 0; i < geom.frames.length; i++) {
        const url = await page.evaluate((i) => window.drawFrame(i), i);
        fs.writeFileSync(path.join(out, `frame_${String(i).padStart(5, '0')}.png`), png(url));
      }
    } else {
      fs.writeFileSync(out, png(await page.evaluate((c) => window.drawSheet(c), geom.cols || 4)));
    }
    if (errors.length) throw new Error(errors.join('\n'));
  } else {
    await page.evaluate(([g, v, s]) => window.render(g, v, s), [geom, viewArg.split(',').map((s) => s.trim()), Number(sizeArg)]);
    if (errors.length) throw new Error(errors.join('\n'));
    await page.locator('#wrap').screenshot({ path: out, omitBackground: !!geom.golden });
  }
} finally {
  await browser.close();
  server.close();
}
