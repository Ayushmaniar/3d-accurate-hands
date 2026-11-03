"""Pass two: build ../clip.json from steps5_sol.json (knot frame), stood up in profile."""
import json, copy, math, os, sys
from hk import *
from steps5 import world, HANDS
from orient import rz, apply

S = {int(k): v for k, v in json.load(open(os.path.join(WORK, 'steps5_sol.json'))).items()}


def shift(hands, d=(0, 0, 0), roll=0.0, pivot=(0.0, 0.0, 0.0)):
    """Move both (world) hands together: turn about world Z through pivot by `roll` degrees
    (positive tips the head, screen right, up), then translate by d."""
    h = copy.deepcopy(hands); m = rz(-roll)
    for s in HANDS:
        pl = h[s]['place']
        w = apply(m, [pl['wrist'][i] - pivot[i] for i in range(3)])
        pl['wrist'] = [round(w[i] + pivot[i] + d[i], 5) for i in range(3)]
        for k in ('palm_facing', 'fingers_toward', 'forearm_toward'):
            pl[k] = [round(x, 5) for x in apply(m, pl[k])]
    return h


W = {n: world(S[n]) for n in S}
# start: palms facing, a few cm apart, relaxed
s0 = copy.deepcopy(S[1])
s0['right']['place']['wrist'][0] -= 0.022; s0['left']['place']['wrist'][0] += 0.022
for s in HANDS:
    for f in ('index', 'middle', 'ring', 'pinky'):
        s0[s][f] = dict(s0[s][f]); s0[s][f]['mcp_flex'] = 8; s0[s][f]['pip_flex'] = 12
    s0[s]['thumb'] = {'cmc_flex': 0, 'cmc_abd': 5, 'cmc_rot': 0, 'mcp_flex': 5, 'ip_flex': 5}
W0 = world(s0)

MOVE = 0.36
keys = [{'t': 0, 'hands': W0}]
for n in range(1, 7):
    k = {'t': float(n), 'move': MOVE, 'ease': 'smooth', 'hands': copy.deepcopy(W[n])}
    if n >= 2: k['collide'] = False
    keys.append(k)
# into the clasp: the hands slide 1 cm past each other with the fingers still straight,
# then the ring fingers (and pinkies) hook over in the last 0.22 s
pre2 = copy.deepcopy(W[2])
for s in HANDS:
    for f in ('ring', 'pinky'):
        pre2[s][f] = dict(S[1][s][f])
keys.append({'t': 1.8, 'move': 0.2, 'ease': 'smooth', 'collide': False, 'hands': pre2})
keys[2]['move'] = 0.2; keys[2]['collide'] = False
# the index fingers and thumbs cross one at a time: the left (near) one first
for n, first in ((4, 'left'), (5, 'right')):
    k = keys[n]; other = 'right' if first == 'left' else 'left'
    k['hands'][first]['move'] = 0.4; k['hands'][other]['move'] = 0.2
# thumbs swing out apart first, then close side by side (they would pass through each other)
mid5 = copy.deepcopy(W[4])
for s in HANDS:
    mid5[s]['thumb'] = {'cmc_flex': -28, 'cmc_abd': -10, 'cmc_rot': 10, 'mcp_flex': 0, 'ip_flex': 0}
keys.append({'t': 4.82, 'move': 0.2, 'ease': 'smooth', 'pass': True, 'collide': False, 'hands': mid5})
keys[5]['hands']['left']['move'] = 0.2; keys[5]['hands']['right']['move'] = 0.2
FWD = -1.0                        # screen right is world -X
rear = shift(W[6], d=(0.028, 0.012, 0.004), roll=8)
keys.append({'t': 6.84, 'move': 0.22, 'ease': 'smooth', 'collide': False, 'hands': rear})
LUNGE = (-0.055, 0.006, 0.012)
strike = shift(W[7], d=LUNGE, roll=-6)
keys.append({'t': 7.0, 'move': 0.14, 'ease': 'smooth', 'collide': False, 'hands': strike})
shut = shift(W[6], d=LUNGE, roll=-6)
keys.append({'t': 7.3, 'move': 0.14, 'ease': 'smooth', 'collide': False, 'hands': shut})
settle = shift(W[6], d=(-0.032, -0.003, 0.006), roll=-3)
keys.append({'t': 7.85, 'move': 0.45, 'ease': 'smooth', 'collide': False, 'hands': settle})

CAM_A = {'position': [-0.15, 0.07, -0.33], 'target': [0.018, 0.008, 0.0], 'fov': 33}
CAM_B = {'position': [-0.19, 0.04, -0.29], 'target': [-0.004, -0.014, 0.0], 'fov': 31}
keys.append({'t': 6.6, 'move': 6.6, 'ease': 'smooth', 'camera': CAM_B})
keys.sort(key=lambda k: k['t'])

clip = {'version': 1, 'name': 'hand_viper',
        'description': 'Two hands fold into a snake head step by step (palms, ring fingers, middle fingers, index snout, thumb jaw, squeeze) and strike.',
        'fps': 30, 'duration': 8.0, 'size': [1920, 1080], 'camera': CAM_A,
        'look': LOOK, 'idle': {'amount': 0.2}, 'keys': keys}
write(os.path.join(WORK, '..', 'clip.json'), clip)
print('wrote clip.json with', len(keys), 'keys')
