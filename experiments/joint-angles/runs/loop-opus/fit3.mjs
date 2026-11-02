import fs from 'fs'; import {mul,mv,add,sub,norm,Rx,Ry,Rz} from './fk.mjs';
const obs=JSON.parse(fs.readFileSync(process.argv[2],'utf8')).map(([f,side,t])=>[JSON.parse(fs.readFileSync(f,'utf8')).hands[side],side,t]);
export function tip(h,side,v,all){const t=h.thumb||{};const W=mul(mul(Ry(h.twist||0),Rx(h.wrist_flex||0)),Rz(-(h.wrist_dev||0)));
 let R=mul(W,mul(mul(mul(Rz(-v.c1),Rx(v.c2)),Ry(v.c3)),mul(mul(mul(Rz(t.cmc_flex||0),Rx(t.cmc_abd||0)),Ry(v.s*(t.cmc_rot||0))),Ry(v.p))));
 let p=mv(W,[0.020,0.018,0.008]);const pts=[p];const L=[0.046,0.034,0.027],q=[0,t.mcp_flex||0,t.ip_flex||0];
 for(let k=0;k<3;k++){R=mul(R,Rx(q[k]));p=add(p,mv(R,[0,L[k],0]));pts.push(p);}
 const [rx,ry,rz]=h.forearm_rotation_deg||[0,0,0];const F=mul(mul(Rx(rx),Ry(ry)),Rz(rz));
 const tf=p=>{p=add([0,0.26,0],p);if(side==='left')p=[-p[0],p[1],p[2]];return add(h.elbow||[0,0,0],mv(F,p));};
 return all?pts.map(tf):tf(p);}
const err=v=>Math.sqrt(obs.reduce((e,[h,side,t])=>e+norm(sub(tip(h,side,v),t))**2,0)/obs.length)*1000;
if(process.argv[1].endsWith('fit3.mjs')){
let res=[];
for(const s of [1,-1])for(let c1=0;c1<=60;c1+=10)for(let c2=-40;c2<=40;c2+=10)for(let c3=-180;c3<180;c3+=15)for(let p=-180;p<180;p+=15){const v={c1,c2,c3,p,s};res.push([err(v),v]);}
res.sort((a,b)=>a[0]-b[0]);
// refine top 20
const out=res.slice(0,20).map(([e,v])=>{for(let st=4;st>0.2;st/=2){let imp=true;while(imp){imp=false;for(const k of ['c1','c2','c3','p'])for(const d of [st,-st]){const w={...v,[k]:v[k]+d};const e2=err(w);if(e2<e){e=e2;v=w;imp=true;}}}}return [e.toFixed(2),v];});
out.sort((a,b)=>a[0]-b[0]);console.log(out.slice(0,8).map(x=>JSON.stringify(x)).join('\n'));}
