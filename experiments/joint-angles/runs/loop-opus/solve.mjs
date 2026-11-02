// generic least-squares over chosen joints: node solve.mjs spec.json
// spec: {pose:file, side, vars:[[path,min,max]], goals:[[finger,jointIdx,[x,y,z],weight]] , out:file}
import fs from 'fs'; import {fk,CFG,sub,norm} from './fk.mjs';

const S=JSON.parse(fs.readFileSync(process.argv[2],'utf8'));
const pose=JSON.parse(fs.readFileSync(S.pose,'utf8'));
const get=(o,p)=>p.split('.').reduce((a,k)=>a?.[k],o);
const set=(o,p,v)=>{const ks=p.split('.');let a=o;for(const k of ks.slice(0,-1))a=a[k]??(a[k]={});a[ks.at(-1)]=v;};
const cost=()=>{let e=0;const J={};const jj=side=>J[side]??=fk(pose.hands[side],side);for(const g of S.goals){if(g[0]==='dist'){const [,a,fa,b,fb,d,w]=g;const pa=jj(a)[fa][3],pb=jj(b)[fb][3];e+=(w??1)*(norm(sub(pa,pb))-d)**2;continue;}if(g[0]==='rel'){const [,a,fa,ia,b,fb,ib,o,w]=g;e+=(w??1)*norm(sub(sub(jj(a)[fa][ia],jj(b)[fb][ib]),o))**2;continue;}if(g[0]==='axis'){const [,sd,fn,ix,ax,val,w]=g;e+=(w??1)*(jj(sd)[fn][ix][ax]-val)**2;continue;}if(g[0]==='line'){/* line from finger base through tip passes point */const [,sd,fn,P,w]=g;const j=jj(sd)[fn];const u=sub(j[3],j[0]);const nu=norm(u);const v=sub(P,j[0]);const t=(v[0]*u[0]+v[1]*u[1]+v[2]*u[2])/nu;const perp=Math.sqrt(Math.max(0,norm(v)**2-t*t));e+=(w??1)*(perp/Math.max(norm(v),1e-3))**2*0.01;continue;}const [side,f,i,t,w]=g;const j=jj(side);const pt=f==='wrist'?j.wrist:j[f][i];e+=(w??1)*norm(sub(pt,t))**2;}return e;};
let c=cost();
for(let step=(S.step??8);step>(S.minstep??0.2);step/=2){let imp=true;while(imp){imp=false;for(const [p,lo,hi,scl] of S.vars){const v0=get(pose,p)??0;for(const d of [step*(scl??1),-step*(scl??1)]){const v=Math.min(hi,Math.max(lo,v0+d));set(pose,p,v);const c2=cost();if(c2<c-1e-12){c=c2;imp=true;break;}set(pose,p,v0);}}}}
for(const [p,,,scl] of S.vars){const r=scl?10000:10;set(pose,p,Math.round(get(pose,p)*r)/r);}
for(const g of S.goals)if(g[0]==='dist'){const pa=fk(pose.hands[g[1]],g[1])[g[2]][3],pb=fk(pose.hands[g[3]],g[3])[g[4]][3];console.log('dist',g[1],g[2],g[3],g[4],(norm(sub(pa,pb))*1000).toFixed(1),'mm');}
console.log('rms mm',(Math.sqrt(cost()/S.goals.length)*1000).toFixed(1));
for(const [p] of S.vars)console.log(p,get(pose,p));
if(S.out)fs.writeFileSync(S.out,JSON.stringify(pose,null,1));
