import fs from 'fs'; import {fk,CFG,sub,norm} from './fk.mjs';
const obs=JSON.parse(process.argv[2]); // [[file, side, [x,y,z]],...]
let best=[];
for (const s of [1,-1]) for (let p=-180;p<180;p+=5){CFG.thumbRotSign=s;CFG.thumbPad0=p;
 let e=0; for(const [f,side,t] of obs){const h=JSON.parse(fs.readFileSync(f,'utf8')).hands[side]; e+=norm(sub(fk(h,side).thumb[3],t))**2;}
 best.push([Math.sqrt(e/obs.length)*1000,s,p]);}
best.sort((a,b)=>a[0]-b[0]); console.log(best.slice(0,6));
