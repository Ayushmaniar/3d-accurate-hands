// Copies the hand meshes (`handkit mesh`, or the first `handkit render --mesh`) into assets/.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const from = path.resolve(process.argv[2] ?? path.join(here, '../../../assets'));
const to = path.join(here, '../assets');
fs.mkdirSync(to, { recursive: true });
for (const f of ['hand_R.glb', 'hand_L.glb']) {
  const src = path.join(from, f);
  if (!fs.existsSync(src)) {
    console.error(`missing ${src}: run \`handkit mesh -o ${src}${f === 'hand_L.glb' ? ' --side left' : ''}\` first`);
    process.exit(1);
  }
  fs.copyFileSync(src, path.join(to, f));
  console.log(`assets/${f}`);
}
