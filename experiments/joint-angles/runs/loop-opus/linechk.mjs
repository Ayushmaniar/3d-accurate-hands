import fs from 'fs'; import {fk,sub,norm} from './fk.mjs';
const p=JSON.parse(fs.readFileSync(process.argv[2],'utf8'));const j=fk(p.hands.right,'right');const P=[-0.6,0.3,-0.4];
const a=j.index[0],b=j.index[3];const u=sub(b,a);const nu=norm(u);const v=sub(P,a);const t=(v[0]*u[0]+v[1]*u[1]+v[2]*u[2])/nu;
console.log('perp dist mm',(Math.sqrt(norm(v)**2-t*t)*1000).toFixed(2),'along',t.toFixed(3));
// also check from pip->tip and using reported tip if given
const tip=process.argv[3]?JSON.parse(process.argv[3]):b;const u2=sub(tip,j.index[1]);const v2=sub(P,j.index[1]);const t2=(v2[0]*u2[0]+v2[1]*u2[1]+v2[2]*u2[2])/norm(u2);
console.log('perp (pip->tip) mm',(Math.sqrt(norm(v2)**2-t2*t2)*1000).toFixed(2));
console.log('mcp',a.map(x=>x.toFixed(4)),'tip',b.map(x=>x.toFixed(4)),'thumbtip',j.thumb[3].map(x=>x.toFixed(3)),'wrist',j.wrist.map(x=>x.toFixed(3)));
