import json, subprocess, sys
sys.path.insert(0,'work')
from proj import fk, project
H="C:/Users/Rajat/Documents/3D Accurate Hands/target/release/handkit.exe"
clip=sys.argv[1]; n=int(sys.argv[2]) if len(sys.argv)>2 else 20
cam=json.load(open(clip))['camera']
prev=None; out=[]
for f in range(0,n):
    t=f/30
    s=subprocess.run([H,'sample',clip,str(t)],capture_output=True,text=True).stdout
    open('work/_s.json','w').write(s); p=fk('work/_s.json')
    r=project(p['right.index.tip'],cam); l=project(p['left.index.tip'],cam)
    r=(r[0]*1080,r[1]*1920); l=(l[0]*1080,l[1]*1920)
    if prev:
        vr=((r[0]-prev[0][0])**2+(r[1]-prev[0][1])**2)**.5; vl=((l[0]-prev[1][0])**2+(l[1]-prev[1][1])**2)**.5
        out.append(f"{vr:.0f}/{vl:.0f}")
    prev=(r,l)
print(' '.join(out))
