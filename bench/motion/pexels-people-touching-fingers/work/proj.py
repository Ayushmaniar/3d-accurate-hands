import sys, json, subprocess, math, re
H="C:/Users/Rajat/Documents/3D Accurate Hands/target/release/handkit.exe"
def fk(posefile):
    out=subprocess.run([H,'fk',posefile],capture_output=True,text=True).stdout
    pts={}
    for line in out.splitlines():
        m=re.match(r'(?:site )?([\w.]+): \[([-\d. ]+)\]',line)
        if m: pts[m.group(1)]=[float(v) for v in m.group(2).split()]
    return pts
def project(p,cam,W=1080,Hh=1920):
    pos=cam['position']; tg=cam['target']; fov=math.radians(cam['fov'])
    f=[tg[i]-pos[i] for i in range(3)]; n=math.sqrt(sum(x*x for x in f)); f=[x/n for x in f]
    up=[0,1,0]
    r=[f[1]*up[2]-f[2]*up[1], f[2]*up[0]-f[0]*up[2], f[0]*up[1]-f[1]*up[0]]; n=math.sqrt(sum(x*x for x in r)); r=[x/n for x in r]
    u=[r[1]*f[2]-r[2]*f[1], r[2]*f[0]-r[0]*f[2], r[0]*f[1]-r[1]*f[0]]
    d=[p[i]-pos[i] for i in range(3)]
    x=sum(d[i]*r[i] for i in range(3)); y=sum(d[i]*u[i] for i in range(3)); z=sum(d[i]*f[i] for i in range(3))
    t=math.tan(fov/2); asp=W/Hh
    return (0.5+x/(z*t*asp)/2, 0.5-y/(z*t)/2)
if __name__=='__main__':
    clip=json.load(open(sys.argv[1])); t=sys.argv[2]
    s=subprocess.run([H,'sample',sys.argv[1],t],capture_output=True,text=True).stdout
    open('_s.json','w').write(s)
    pose=json.loads(s); cam=pose.get('camera',clip.get('camera'))
    pts=fk('_s.json')
    for k in ['right.forearm','right.hand','right.index.tip','right.thumb.tip','right.middle.tip','left.forearm','left.hand','left.index.tip','left.thumb.tip','left.middle.tip']:
        if k in pts:
            x,y=project(pts[k],cam); print(f"{k:18s} ({x:.3f}, {y:.3f})  world {pts[k]}")
