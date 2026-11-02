import json, subprocess, numpy as np, copy, sys
H='../../../target/release/handkit'
def nrm(v): v=np.array(v,float); return v/np.linalg.norm(v)
def sample_place(hand, place, scale=0.75):
    h=copy.deepcopy(hand); h['place']=place
    c={'version':1,'fps':30,'duration':1.0,'size':[1920,1080],'idle':{'amount':0},
       'camera':{'position':[0,0.14,0.6],'target':[0,-0.015,0],'fov':28},
       'people':{'kid':{'scale':scale}},'keys':[{'t':0,'hands':{'kid.right':h}}]}
    json.dump(c,open('_c.json','w'))
    s=json.loads(subprocess.run([H,'sample','_c.json','0.5','--wrist'],capture_output=True,text=True).stdout)
    return s['hands']['kid.right']['place']
def correct(hand, want, iters=6):
    want=copy.deepcopy(want); f=nrm(want['fingers_toward']); p=nrm(want['palm_facing']); p=nrm(p-p.dot(f)*f)
    want['palm_facing']=list(p); want['fingers_toward']=list(f)
    key=copy.deepcopy(want)
    for i in range(iters):
        got=sample_place(hand,key)
        dw=np.array(want['wrist'])-np.array(got['wrist'])
        dp=nrm(want['palm_facing'])-nrm(got['palm_facing'])
        df=nrm(want['fingers_toward'])-nrm(got['fingers_toward'])
        err=np.linalg.norm(dw)*1000, np.degrees(np.linalg.norm(dp)), np.degrees(np.linalg.norm(df))
        if max(err)<0.5: break
        key['wrist']=list(np.array(key['wrist'])+dw)
        key['palm_facing']=list(nrm(nrm(key['palm_facing'])+dp))
        key['fingers_toward']=list(nrm(nrm(key['fingers_toward'])+df))
    print('residual mm/deg/deg', [round(e,2) for e in err], file=sys.stderr)
    return {k:[round(float(x),5) for x in v] for k,v in key.items()}
if __name__=='__main__':
    hs=json.load(open('hs3_sol.json'))['hands']['kid.right']
    print(correct(hs, hs['place']))
