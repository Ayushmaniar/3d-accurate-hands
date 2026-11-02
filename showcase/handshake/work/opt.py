import json, subprocess, math, random, sys, copy
H='../../../target/release/handkit'
mode=sys.argv[1]  # hs or fb
g=json.load(open('../../../crates/handkit-core/gestures/handshake.json'))
dad0=g['hands']['ana.right']; kid0=g['hands']['ben.right']
if mode=='hs':
    x={'kx':0.07,'ky':-0.02,'kz':-0.036,'kth':-25.0,'kpx':0.0,'kpy':-0.05,'km':15,'kp':25,'dm':30,'dp':55,'kroll':0.0}
    steps={'kx':0.006,'ky':0.006,'kz':0.005,'kth':6,'kpx':0.1,'kpy':0.1,'km':8,'kp':10,'dm':8,'dp':10,'kroll':0.1}
    goals=[('kid.right.palm_center','dad.right.middle.metacarpal',2),
           ('dad.right.thumb.distal','kid.right.index.metacarpal',1),
           ('kid.right.thumb.distal','dad.right.index.metacarpal',1),
           ('dad.right.middle.distal','kid.right.pinky.metacarpal',1),
           ('dad.right.ring.distal','kid.right.pinky.metacarpal',0.7),
           ('kid.right.middle.distal','dad.right.pinky.metacarpal',0.5)]
def build(x):
    dad=copy.deepcopy(dad0); kid=copy.deepcopy(kid0)
    for i,f in enumerate(('index','middle','ring','pinky')):
        dad[f]['mcp_flex']=x['dm']+4*i; dad[f]['pip_flex']=x['dp']+5*i; dad[f]['dip_flex']=0.6*(x['dp']+5*i)
        kid[f]['mcp_flex']=x['km']+5*i; kid[f]['pip_flex']=x['kp']+8*i; kid[f]['dip_flex']=0.6*(x['kp']+8*i)
    th=math.radians(x['kth'])
    ft=[-math.cos(th)*1, math.sin(th), x['kroll']]
    kid['place']={'wrist':[x['kx'],x['ky'],x['kz']],'palm_facing':[x['kpx'],x['kpy'],1],'fingers_toward':ft,'forearm_toward':[0.92,0.3,-0.24]}
    return {'version':1,'hands':{'dad.right':dad,'kid.right':kid},'people':{'dad':{},'kid':{'scale':0.75}},
            'goals':[{'touch':[a,b]} for a,b,w in goals]}
def score(x):
    p=build(x); json.dump(p,open('_o.json','w'))
    q=json.loads(subprocess.run([H,'query','_o.json','--json'],capture_output=True,text=True).stdout)
    s=0
    for gq,(a,b,w) in zip(q['goals'],goals): s+=w*abs(gq['value'])
    for pe in q['penetrations']: s+=2*abs(pe['gap_mm'])
    return s
best=score(x); print('start',best)
for it in range(int(sys.argv[2])):
    improved=False
    for k in x:
        for sgn in (1,-1):
            y=dict(x); y[k]+=sgn*steps[k]; s=score(y)
            if s<best: best,x,improved=s,y,True; break
    if not improved:
        for k in steps: steps[k]*=0.6
    print(it,round(best,1),flush=True)
json.dump(x,open(mode+'_x.json','w'),indent=1)
json.dump(build(x),open(mode+'_opt.json','w'),indent=1)
