import {fk,sub,norm} from './fk.mjs';
const D=180/Math.PI;let res=[];
for(let a=-10;a<=40;a+=5)for(let r=-15;r<=30;r+=15)for(const m of [-15,-5,0])for(const i of [-20,-10,0]){
 const h={thumb:{cmc_flex:-20,cmc_abd:a,cmc_rot:r,mcp_flex:m,ip_flex:i}};const j=fk(h,'right');
 const t=sub(j.thumb[3],j.thumb[0]);const ix=[Math.sin(4/D)*0,1,0];
 const ang=Math.acos(t[1]/norm(t))*D;res.push([ang.toFixed(0),(t[2]/norm(t)).toFixed(2),a,r,m,i]);}
res.sort((x,y)=>y[0]-x[0]);console.log(res.slice(0,12));
