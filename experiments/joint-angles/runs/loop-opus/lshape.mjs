import {fk,sub,norm} from './fk.mjs';
const D=180/Math.PI;
for (const [f,a,m,i] of [[-20,0,0,0],[-20,10,-15,0],[-20,20,-15,-10],[-20,30,0,0],[-10,40,0,0],[-20,40,-10,0]]){
 const h={thumb:{cmc_flex:f,cmc_abd:a,mcp_flex:m,ip_flex:i},middle:{mcp_flex:85,pip_flex:100,dip_flex:60},ring:{mcp_flex:85,pip_flex:100,dip_flex:60},pinky:{mcp_flex:85,pip_flex:100,dip_flex:60}};
 const j=fk(h,'right'); const t=sub(j.thumb[3],j.thumb[1]); const ix=sub(j.index[3],j.index[0]);
 const ang=Math.acos((t[0]*ix[0]+t[1]*ix[1]+t[2]*ix[2])/norm(t)/norm(ix))*D;
 console.log(f,a,m,i,'angle',ang.toFixed(0),'thumb dir',t.map(x=>(x/norm(t)).toFixed(2)).join(','),'tip',j.thumb[3].map(x=>x.toFixed(3)).join(','));
}
