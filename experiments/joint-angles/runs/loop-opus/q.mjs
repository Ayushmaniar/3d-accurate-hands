// quick: node q.mjs file.json [side] -> all joints
import fs from 'fs'; import {fk,CFG} from './fk.mjs';

const p=JSON.parse(fs.readFileSync(process.argv[2],'utf8'));
for(const [side,h] of Object.entries(p.hands)){const j=fk(h,side);console.log(side,'wrist',j.wrist.map(x=>x.toFixed(3)).join(','));
for(const n of ['thumb','index','middle','ring','pinky'])console.log(' ',n.padEnd(6),j[n].map(v=>'('+v.map(x=>x.toFixed(3)).join(',')+')').join(' '));}
