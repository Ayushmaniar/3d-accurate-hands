import json, subprocess, math, sys, copy
H='../../../target/release/handkit'
g=json.load(open('../../../crates/handkit-core/gestures/handshake.json'))
dad0=g['hands']['ana.right']; kid0=g['hands']['ben.right']
x=json.load(open('hs2_x.json'))
steps={'kx':0.006,'ky':0.006,'kz':0.006,'kc':0.2,'kmc':0.3,'dc':0.2,'dmc':0.3,'kth':5,'kpx':0.08,'kpy':0.08}
goals=[('dad.right.middle.distal','kid.right.pinky.metacarpal',0.5),
  ('dad.right.thumb.distal','kid.right.index.metacarpal',0.5),
  ('kid.right.thumb.distal','dad.right.index.metacarpal',0.5),
  ('kid.right.palm_center','dad.right.middle.metacarpal',4.0)]
def build(x):
    dad=copy.deepcopy(dad0); kid=copy.deepcopy(kid0)
    for f in ('middle','ring','pinky'):
        dad[f]['mcp_flex']*=x['dmc']; dad[f]['pip_flex']*=x['dc']; dad[f]['dip_flex']*=x['dc']
        kid[f]['mcp_flex']*=x['kmc']; kid[f]['pip_flex']*=x['kc']; kid[f]['dip_flex']*=x['kc']
    kid['index']['pip_flex']*=x['kc']; kid['index']['dip_flex']*=x['kc']
    for h in (dad,kid):
        for f in ('index','middle','ring','pinky'):
            h[f]['mcp_flex']=max(-20,min(90,h[f]['mcp_flex'])); h[f]['pip_flex']=max(0,min(105,h[f]['pip_flex'])); h[f]['dip_flex']=max(0,min(80,h[f]['dip_flex']))
    th=math.radians(x['kth']); fx,fy=-0.964,-0.3
    ft=[fx*math.cos(th)-fy*math.sin(th), fx*math.sin(th)+fy*math.cos(th), 0]
    kid['place']={'wrist':[x['kx'],x['ky'],x['kz']],'palm_facing':[x['kpx'],x.get('kpy',-0.025),1],'fingers_toward':ft,'forearm_toward':[0.92,0.3,-0.24]}
    return {'version':1,'hands':{'dad.right':dad,'kid.right':kid},'people':{'dad':{},'kid':{'scale':0.75}},
            'goals':[{'touch':[a,b]} for a,b,w in goals]}
def score(x):
    json.dump(build(x),open('_o.json','w'))
    q=json.loads(subprocess.run([H,'query','_o.json','--json'],capture_output=True,text=True).stdout)
    s=sum(w*abs(gq['value']) for gq,(a,b,w) in zip(q['goals'],goals))
    s+=sum(2*abs(pe['gap_mm']) for pe in q['penetrations'])
    return s
best=score(x); print('start',best)
for it in range(int(sys.argv[1])):
    improved=False
    for k in x:
        for sgn in (1,-1):
            y=dict(x); y[k]+=sgn*steps[k]; s=score(y)
            if s<best: best,x,improved=s,y,True
    if not improved:
        for k in steps: steps[k]*=0.6
    print(it,round(best,1),flush=True)
json.dump(x,open('hs2_x.json','w'),indent=1)
json.dump(build(x),open('hs2_opt.json','w'),indent=1)
