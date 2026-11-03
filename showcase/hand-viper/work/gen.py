"""Build ../clip.json from the solved step poses (steps_sol.json) and the strike poses."""
import json, copy, math, os, sys
from hk import *

S = json.load(open(os.path.join(WORK, 'steps_sol.json')))
HANDS = ('right', 'left')


def rot_x(v, deg):
    a = math.radians(deg); c, s = math.cos(a), math.sin(a)
    return [v[0], c * v[1] - s * v[2], s * v[1] + c * v[2]]


def move(hands, d=(0, 0, 0), pitch=0.0, pivot=(0, 0.07, 0)):
    """Rigidly move both hands together: pitch about X through pivot (negative tips the
    fingers toward the camera at -Z), then translate by d."""
    h = copy.deepcopy(hands)
    for side in HANDS:
        pl = h[side]['place']
        w = [pl['wrist'][i] - pivot[i] for i in range(3)]
        w = rot_x(w, pitch)
        pl['wrist'] = [round(w[i] + pivot[i] + d[i], 5) for i in range(3)]
        for k in ('palm_facing', 'fingers_toward', 'forearm_toward'):
            pl[k] = [round(x, 5) for x in rot_x(pl[k], pitch)]
    return h


def apart(hands, dx):
    h = copy.deepcopy(hands)
    h['right']['place']['wrist'][0] -= dx; h['left']['place']['wrist'][0] += dx
    return h


def jaw(hands, thumb):
    h = copy.deepcopy(hands)
    for side in HANDS:
        h[side]['thumb'] = dict(thumb)
    return h


S7 = json.load(open(os.path.join(WORK, 'j2_sol.json')))['hands'] if os.path.exists(os.path.join(WORK, 'j2_sol.json')) else None

# start: palms facing, a little apart, fingers relaxed
s0 = apart(S['1'], 0.03)
for side in HANDS:
    for f in ('index', 'middle', 'ring', 'pinky'):
        s0[side][f] = dict(s0[side][f]); s0[side][f]['mcp_flex'] = 6; s0[side][f]['pip_flex'] = 10
    s0[side]['thumb'] = {'cmc_flex': 0, 'cmc_abd': 5, 'cmc_rot': 0, 'mcp_flex': 5, 'ip_flex': 5}

MOVE = 0.36
IDX = {'touch': ['right.index.tip', 'left.index.tip'], 'vary': ['right.index.*', 'left.index.*']}
keys = []
keys.append({'t': 0, 'hands': s0})
GOALS = {n: json.load(open(os.path.join(WORK, 'st%d_sol.json' % n))).get('goals', []) for n in range(1, 7)}
USE_GOALS = '--goals' in sys.argv
for n in range(1, 7):
    k = {'t': float(n), 'move': MOVE, 'ease': 'smooth', 'hands': copy.deepcopy(S[str(n)])}
    if n >= 3: k['collide'] = False
    if USE_GOALS and n >= 2: k['goals'] = GOALS[n]

    keys.append(k)
k2 = keys[2]; k2['move'] = 0.42          # the ring fingers need the V to open first
k3 = keys[3]; k3['hands']['left']['move'] = 0.42; k3['hands']['right']['move'] = 0.26   # left middle first, right lands on top
# strike: rear back, then snap forward with the jaw open, shut it, settle
rear = move(S['6'], d=(0, 0.012, 0.035), pitch=12)
keys.append({'t': 6.84, 'move': 0.2, 'ease': 'smooth', 'hands': rear})
strike = S7 if S7 else move(S['6'], d=(0, 0.026, -0.07), pitch=-12)
keys.append({'t': 7.0, 'move': 0.13, 'ease': 'smooth', 'hands': strike})
shut = move(S['6'], d=(0, 0.026, -0.07), pitch=-12)
keys.append({'t': 7.32, 'move': 0.12, 'ease': 'snappy', 'hands': shut})
settle = move(S['6'], d=(0, 0.012, -0.045), pitch=-10)
keys.append({'t': 7.85, 'move': 0.4, 'ease': 'smooth', 'hands': settle})

CAM_A = {'position': [-0.035, 0.27, -0.42], 'target': [0, 0.105, 0], 'fov': 29}
CAM_B = {'position': [0.07, 0.19, -0.41], 'target': [0, 0.095, -0.01], 'fov': 30}
keys.append({'t': 6.6, 'move': 6.6, 'ease': 'smooth', 'camera': CAM_B})
keys.sort(key=lambda k: k['t'])
for k in keys:
    if k['t'] >= 3 and 'hands' in k: k['collide'] = False

clip = {'version': 1, 'name': 'hand_viper',
        'description': 'Two hands fold into a snake head step by step (palms, ring fingers crossed, middle on top, index snout, thumb jaw, squeeze) and strike.',
        'fps': 30, 'duration': 8.0, 'size': [1920, 1080], 'camera': CAM_A,
        'look': LOOK, 'idle': {'amount': 0.25}, 'keys': keys}
write(os.path.join(WORK, '..', 'clip.json'), clip)
print('wrote clip.json with', len(keys), 'keys')
