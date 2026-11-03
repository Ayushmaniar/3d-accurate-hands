import json, copy
from hk import *
from gen import move, S
cam = {'position': [0.045, 0.22, -0.41], 'target': [0, 0.10, -0.01], 'fov': 29}
base = move(S['6'], d=(0, 0.026, -0.07), pitch=-12)
print(base['right']['thumb'], base['left']['thumb'])
cands = [{'cmc_flex': -25, 'cmc_abd': 10, 'cmc_rot': 0, 'mcp_flex': 0, 'ip_flex': 10},
         {'cmc_flex': -30, 'cmc_abd': 25, 'cmc_rot': 10, 'mcp_flex': 10, 'ip_flex': 20},
         {'cmc_flex': -15, 'cmc_abd': 0, 'cmc_rot': -10, 'mcp_flex': 0, 'ip_flex': 30}]
for i, th in enumerate(cands):
    h = copy.deepcopy(base)
    for sd in ('right', 'left'): h[sd]['thumb'] = th
    g = [{'vary': ['right.thumb.*', 'left.thumb.*']}]
    c, t, s = solve('j%d' % i, h, g)
    q = query('j%d_sol.json' % i)
    print(i, s['hands']['right']['thumb'], q[q.index('penetrations'):q.index('contacts')])
    preview('j%d' % i, s['hands'], camera=cam, width=480)
