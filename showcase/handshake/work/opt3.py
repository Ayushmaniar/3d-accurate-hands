import json, subprocess, sys, copy
H='../../../target/release/handkit'
g=json.load(open('../../../crates/handkit-core/gestures/fist_bump_pair.json'))
dad0=g['hands']['ana.right']; kid0=g['hands']['ben.right']
x={'kx':0.075,'ky':-0.005,'kz':-0.035}
steps={'kx':0.008,'ky':0.006,'kz':0.008}
goals=[('dad.right.middle.proximal','kid.right.middle.proximal',1),('dad.right.index.proximal','kid.right.index.proximal',1),('dad.right.ring.proximal','kid.right.ring.proximal',0.5)]
def build(x):
    dad=copy.deepcopy(dad0); kid=copy.deepcopy(kid0)
    kid['place']['wrist']=[x['kx'],x['ky'],x['kz']]
    return {'version':1,'hands':{'dad.right':dad,'kid.right':kid},'people':{'dad':{},'kid':{'scale':0.75}},'goals':[{'touch':[a,b]} for a,b,w in goals]}
def score(x):
    json.dump(build(x),open('_f.json','w'))
    q=json.loads(subprocess.run([H,'query','_f.json','--json'],capture_output=True,text=True).stdout)
    return sum(w*abs(gq['value']) for gq,(a,b,w) in zip(q['goals'],goals))+sum(3*abs(p['gap_mm']) for p in q['penetrations'])
best=score(x)
for it in range(40):
    imp=False
    for k in x:
        for s in (1,-1):
            y=dict(x); y[k]+=s*steps[k]; v=score(y)
            if v<best: best,x,imp=v,y,True
    if not imp:
        for k in steps: steps[k]*=0.6
print(best,x); json.dump(build(x),open('fb_opt.json','w'),indent=1)
