import fs from 'fs'; import {mul,mv,add,sub,norm,Rx,Ry,Rz} from './fk.mjs';
const obs=JSON.parse(fs.readFileSync(process.argv[2],'utf8'));
// thumb tip in hand-local coordinates, then same wrist/forearm transform as fk
function tip(h,side,v){const t=h.thumb||{};const tw=h.twist||0,fl=h.wrist_flex||0,dv=h.wrist_dev||0;
 const W=mul(mul(Ry(tw),Rx(fl)),Rz(-dv));
 let R;const f=t.cmc_flex||0,a=t.cmc_abd||0,r=v.s*(t.cmc_rot||0)+v.p;
 if(v.ord==='A')R=mul(mul(mul(Rz(-v.lean),Rz(f)),Rx(a)),Ry(r));
 if(v.ord==='B')R=mul(mul(mul(Rx(a),Rz(-v.lean)),Rz(f)),Ry(r));
 if(v.ord==='C')R=mul(mul(mul(Rz(-v.lean),Rx(a)),Rz(f)),Ry(r));
 if(v.ord==='D')R=mul(mul(mul(Rz(-v.lean),Rz(f)),Ry(v.p)),mul(Rx(a),Ry(v.s*(t.cmc_rot||0))));
 let p=[0.020,0.018,0.008];R=mul(W,R);p=mv(W,p);const L=[0.046,0.034,0.027],q=[0,t.mcp_flex||0,t.ip_flex||0];
 for(let k=0;k<3;k++){R=mul(R,Rx(q[k]));p=add(p,mv(R,[0,L[k],0]));}
 p=add([0,0.26,0],p); if(side==='left')p[0]=-p[0];
 const [rx,ry,rz]=h.forearm_rotation_deg||[0,0,0];return add(h.elbow||[0,0,0],mv(mul(mul(Rx(rx),Ry(ry)),Rz(rz)),p));}
let res=[];
for(const ord of ['A','B','C','D'])for(let lean=0;lean<=70;lean+=5)for(const s of [1,-1])for(let p=-180;p<180;p+=5){const v={ord,lean,s,p};let e=0;
 for(const [f,side,t] of obs){const h=JSON.parse(fs.readFileSync(f,'utf8')).hands[side];e+=norm(sub(tip(h,side,v),t))**2;}
 res.push([+(Math.sqrt(e/obs.length)*1000).toFixed(2),ord,lean,s,p]);}
res.sort((a,b)=>a[0]-b[0]);console.log(res.slice(0,10));
export {tip};
