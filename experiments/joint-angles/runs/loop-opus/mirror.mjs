// node mirror.mjs file.json : copies right hand to left (mirrored) and sets ring/pinky from middle if missing
import fs from 'fs';
const f=process.argv[2];const p=JSON.parse(fs.readFileSync(f,'utf8'));const r=p.hands.right;
const L=JSON.parse(JSON.stringify(r));L.elbow=[-r.elbow[0],r.elbow[1],r.elbow[2]];
const [a,b,c]=r.forearm_rotation_deg||[0,0,0];L.forearm_rotation_deg=[a,-b,-c];
p.hands.left=L;fs.writeFileSync(f,JSON.stringify(p,null,1));
