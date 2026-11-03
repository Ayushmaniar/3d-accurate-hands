import json, copy, sys
import os
hs=json.load(open('work/hs3_sol.json'))['hands']
# The library's fist_bump grip, solved between the two sizes of hand (fb_grip_sol.json).
fb=json.load(open('work/fb_grip_sol.json'))['hands']
def hand(src, place=None):
    h={k:v for k,v in copy.deepcopy(src).items() if k!='place'}
    if place: h['place']=place
    return h
def P(w, pf, ft, fa): return {'wrist':w,'palm_facing':pf,'fingers_toward':ft,'forearm_toward':fa}
dHS=hs['dad.right']['place']; kHS=hs['kid.right']['place']
def shift(pl, d):
    q=copy.deepcopy(pl); q['wrist']=[a+b for a,b in zip(q['wrist'],d)]; return q
openthumb={'from':'open_palm','thumb':{'cmc_flex':-25,'cmc_abd':40,'cmc_rot':0,'mcp_flex':0,'ip_flex':0}}
dREST=P([-0.31,-0.08,0.04],[0.2,0.3,-1],[1,-0.6,0],[-1,0.1,0.3])
kREST=P([0.27,-0.11,-0.07],[-0.2,0.3,1],[-1,-0.5,0],[1,-0.05,-0.3])
dFB=fb['dad.right']['place']; kFB=fb['kid.right']['place']
# The grip touches at the collision capsules; the skin over the knuckles sits inside them, so the
# fists come TOUCH closer for the skin to meet, and PRESS more on the hit.
TOUCH=float(os.environ.get('TOUCH','0.009')); PRESS=0.003
def closed(pl,side,d): return shift(pl,[side*d/2,0,0])
keys=[]
keys.append({'t':0,'hands':{'dad.right':{'from':'relaxed','place':dREST},'kid.right':{'from':'relaxed','place':kREST}}})
# approach
keys.append({'t':0.75,'pass':True,'hands':{
  'dad.right':dict(openthumb,place=shift(dHS,[-0.045,-0.005,0.04])),
  'kid.right':dict(openthumb,place=shift(kHS,[0.04,-0.005,-0.04]))}})
keys.append({'t':0.98,'hands':{'dad.right':{'place':shift(dHS,[-0.012,0,0.018])},'kid.right':{'place':shift(kHS,[0.01,0,-0.018])}}})
keys.append({'t':1.22,'cascade':0.03,'hands':{'dad.right':hand(hs['dad.right'],dHS),'kid.right':hand(hs['kid.right'],kHS)}})
# pumps: both hands translate together
for t,dy in [(1.47,0.024),(1.74,-0.02),(2.01,0.022),(2.28,-0.016),(2.55,0.0)]:
    k={'t':t,'hands':{'dad.right':{'place':shift(dHS,[0,dy,0])},'kid.right':{'place':shift(kHS,[0,dy,0])}}}
    if dy!=0: k['pass']=True
    keys.append(k)
keys.append({'t':2.8,'hands':{'dad.right':{},'kid.right':{}}})
keys.append({'t':3.02,'hands':{'dad.right':dict(openthumb,place=shift(dHS,[-0.015,0,0.03])),'kid.right':dict(openthumb,place=shift(kHS,[0.012,0,-0.03]))}})
# release
keys.append({'t':3.3,'pass':True,'hands':{
  'dad.right':dict(openthumb,place=shift(dHS,[-0.075,-0.005,0.045])),
  'kid.right':dict(openthumb,place=shift(kHS,[0.065,-0.005,-0.045]))}})
# fists ready, apart
keys.append({'t':3.7,'hands':{
  'dad.right':hand(fb['dad.right'],shift(dFB,[-0.065,0.02,0])),
  'kid.right':hand(fb['kid.right'],shift(kFB,[0.05,0.015,0]))}})
# bump
keys.append({'t':4.1,'move':0.2,'ease':'snappy','hands':{
  'dad.right':{'place':closed(dFB,1,TOUCH+PRESS)},'kid.right':{'place':closed(kFB,-1,TOUCH+PRESS)}}})
keys.append({'t':4.22,'move':0.12,'ease':'smooth','hands':{
  'dad.right':{'place':closed(dFB,1,TOUCH)},'kid.right':{'place':closed(kFB,-1,TOUCH)}}})
keys.append({'t':4.4,'hands':{'dad.right':{},'kid.right':{}}})
# recoil
keys.append({'t':4.75,'ease':'smooth','hands':{
  'dad.right':{'place':shift(dFB,[-0.07,0.015,0])},'kid.right':{'place':shift(kFB,[0.06,0.012,0])}}})
keys.append({'t':5.5,'hands':{'dad.right':{'from':'relaxed','place':dREST},'kid.right':{'from':'relaxed','place':kREST}}})
clip={'version':1,'name':'handshake_fistbump','description':'An adult and a child shake hands, let go, then fist-bump.',
 'fps':30,'duration':6.0,'size':[1920,1080],
 'camera':{'position':[0.0,0.14,0.6],'target':[0.0,-0.015,0.0],'fov':28},
 'look':{'style':'realistic'},'idle':{'amount':0.25},'collide':False,
 'people':{'dad':{'look':{'skin':'brown','arm':{'sleeve':'#2b3a55','length':0.09}}},
           'kid':{'scale':0.75,'look':{'skin':'light','arm':{'sleeve':'#d9a23a','length':0.07}}}},
 'keys':keys}
json.dump(clip,open('clip.json','w'),indent=1)
