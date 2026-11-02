// Renders a pose in every look with @handkit/three and puts it under the fframes renders:
//   node tools/looks/compare.mjs <pose.json> <ffdir> <out.png> [size=1080]
// <ffdir> holds ff_<style>.png from tools/looks/ff_cells.sh (the fframes `looks` example, one
// style at a time, medium skin). Writes <out.png> (row fframes, row three.js) and
// three_<style>.png next to it. Needs `npm install` here and a Playwright Chromium.
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright-core';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '../..');
const [poseFile, ffDir, out, sizeArg = '1080'] = process.argv.slice(2);
if (!poseFile || !ffDir || !out) { console.error('usage: node compare.mjs <pose.json> <ffdir> <out.png> [size]'); process.exit(1); }
const size = Number(sizeArg);
const STYLES = ['realistic', 'toon', 'spiderverse', 'arcane', 'clay'];

const types = { '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript', '.wasm': 'application/wasm', '.glb': 'model/gltf-binary', '.json': 'application/json' };
const server = http.createServer((req, res) => {
  const file = path.join(root, decodeURIComponent(req.url.split('?')[0]));
  if (!file.startsWith(root) || !fs.existsSync(file) || fs.statSync(file).isDirectory()) { res.writeHead(404); return res.end(); }
  res.writeHead(200, { 'content-type': types[path.extname(file)] || 'application/octet-stream' });
  fs.createReadStream(file).pipe(res);
}).listen(0);

export function findChrome() {
  if (process.env.HANDKIT_CHROME) return process.env.HANDKIT_CHROME;
  const base = path.join(process.env.LOCALAPPDATA || path.join(process.env.HOME || '', '.cache'), 'ms-playwright');
  const dirs = fs.readdirSync(base).filter((d) => /^chromium-\d+$/.test(d)).sort().reverse();
  for (const d of dirs) for (const sub of ['chrome-win64/chrome.exe', 'chrome-win/chrome.exe', 'chrome-linux/chrome', 'chrome-mac/Chromium.app/Contents/MacOS/Chromium']) {
    const exe = path.join(base, d, sub);
    if (fs.existsSync(exe)) return exe;
  }
  throw new Error(`no Playwright Chromium under ${base}; set HANDKIT_CHROME`);
}

// The GPU when there is one (HANDKIT_GL=swiftshader for the CPU).
const gl = process.env.HANDKIT_GL === 'swiftshader' ? ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'] : ['--use-angle=default', '--ignore-gpu-blocklist', '--enable-gpu'];
const browser = await chromium.launch({ executablePath: findChrome(), args: gl });
try {
  const page = await browser.newPage({ viewport: { width: size, height: size } });
  const errors = [];
  page.on('pageerror', (e) => errors.push(e.message));
  page.on('console', (m) => { if (m.type() === 'error' || m.type() === 'warning') console.error('page:', m.text()); });
  await page.goto(`http://localhost:${server.address().port}/tools/looks/compare.html`);
  await page.waitForFunction(() => window.ready === true, null, { timeout: 30000 });
  await page.evaluate(([p, s]) => window.setup(p, s), [fs.readFileSync(poseFile, 'utf8'), size]);
  const renderer = await page.evaluate(() => { const gl = document.querySelector('canvas').getContext('webgl2'); const d = gl.getExtension('WEBGL_debug_renderer_info'); return d ? gl.getParameter(d.UNMASKED_RENDERER_WEBGL) : '?'; });
  console.log(`renderer: ${renderer}`);
  const cells = {};
  const png = (url) => Buffer.from(url.slice(url.indexOf(',') + 1), 'base64');
  const dataUrl = (file) => `data:image/png;base64,${fs.readFileSync(file).toString('base64')}`;
  for (const s of STYLES) {
    const r = await page.evaluate((s) => window.renderStyle(s), s);
    if (errors.length) throw new Error(errors.join('\n'));
    console.log(`${s}: first frame ${r.first.toFixed(0)} ms (bakes its textures, compiles shaders), then ${r.second.toFixed(1)} ms per frame at ${size}px`);
    fs.writeFileSync(path.join(path.dirname(out), `three_${s}.png`), png(r.url));
    cells[s] = { fframes: dataUrl(path.join(ffDir, `ff_${s}.png`)), three: r.url };
  }
  const title = `${path.basename(poseFile)}, medium skin`;
  fs.writeFileSync(out, png(await page.evaluate(([c, s, t]) => window.sheet(c, s, t), [cells, size, title])));
  console.log(`wrote ${out}`);
} finally {
  await browser.close();
  server.close();
}
