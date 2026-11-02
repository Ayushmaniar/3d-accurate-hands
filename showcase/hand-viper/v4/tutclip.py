"""The hand viper as youtube ef1n1x2WlZ0 makes it, facing the camera, 10 s: the tutorial's steps
(steps.py) a second apart, the hands cupping into the head (mouth2.py), the mouth opening and
shutting (the thumbs leaving and meeting the index tips), then a strike."""
import copy
from steps import *
from mouth import head_open, ARCH, SHUT
from mouth2 import shut_clear

LOOK_CLIP = {'style': 'realistic', 'skin': 'medium', 'motion_blur': 3, 'arm': {'sleeve': '#262a33', 'length': 0.12}}
CAM = {'position': [0.0, 0.08, 0.50], 'target': [0.0, 0.08, 0.0], 'fov': 32}
HEAD_CAM = {'position': [0.0, 0.09, 0.40], 'target': [0.0, 0.085, 0.0], 'fov': 28}

s1, s2, s3, s4a, s4, s5 = build()
# The head keeps step 5's knot (as the tutorial's does): only the thumbs come up to the index tips.
# As the tutorial's head: the wrists close in a V and the hands leaning out above them, the
# index fingers curled into a low round arch on top, the knot folded in under it, the thumbs
# bent short at the bottom of the mouth (headsweep2.py chose these).
h = copy.deepcopy(s5)
pl = oriented(-5, 0, -45, 0.036)
for s in S:
    h[s]['place'] = dict(pl[s]['place'], forearm_toward=h[s]['place']['forearm_toward'])
    h[s]['thumb'] = {'cmc_flex': 30, 'cmc_abd': 35, 'cmc_rot': 35, 'mcp_flex': 30, 'ip_flex': 40}
    for d in ('middle', 'ring'):
        h[s][d] = dict(h[s][d], mcp_flex=min(90, h[s][d]['mcp_flex'] + 50))
    h[s]['index'] = {'mcp_abd': -5, 'mcp_roll': 0, 'mcp_flex': 35, 'pip_flex': 90, 'dip_flex': 45}
code, text, sol = solve('tut_head', {'version': 1, 'hands': h, 'goals': [ARCH] + SHUT + [clear('middle', 'ring', 'pinky')]})
print('head', [l.strip()[:80] for l in text.splitlines() if 'MISS' in l or 'penetr' in l])
head = sol['hands']


def me(h):
    return {'me.' + s: copy.deepcopy(v) for s, v in h.items()}


def moved(hands, dy=0.0, dz=0.0):
    out = copy.deepcopy(hands)
    for s in out:
        w = out[s]['place']['wrist']
        out[s]['place']['wrist'] = [w[0], round(w[1] + dy, 4), round(w[2] + dz, 4)]
    return out


def held(goals):
    return [json.loads(json.dumps(g).replace('"right.', '"me.right.').replace('"left.', '"me.left.')) for g in goals]


SHUT_HELD, OPEN_HELD = held([ARCH] + SHUT), held([ARCH])
keys = [{'t': 0.0, 'hands': me(s1)}]
for t, h in ((1.0, s2), (2.0, s3), (3.0, s4a), (3.9, s4), (5.0, s5)):
    keys.append({'t': t, 'hands': me(h), 'move': 0.6, 'ease': 'smooth'})
keys += [
    {'t': 5.0, 'camera': CAM},
    {'t': 6.2, 'camera': HEAD_CAM, 'move': 1.2, 'ease': 'smooth'},
    # The head poses are solved clear of each other: from here the clip keeps them as solved.
    {'t': 6.2, 'hands': me(head), 'move': 0.9, 'ease': 'smooth'},
    {'t': 7.1, 'hands': me(head_open(head, 0.7)), 'move': 0.3, 'ease': 'snappy'},
    {'t': 7.6, 'hands': me(head), 'move': 0.25, 'ease': 'snappy'},
    {'t': 8.5, 'hands': me(moved(head_open(head, 1.0), 0.01, -0.035)), 'move': 0.5, 'ease': 'smooth'},
    {'t': 8.8, 'hands': me(moved(head, -0.005, 0.045)), 'move': 0.16, 'ease': 'smooth'},
    {'t': 9.4, 'hands': me(head), 'move': 0.5},
]
for k in keys:
    if 'hands' in k and k['t'] >= 7.1:
        k['collide'] = False

clip = {'version': 1, 'fps': 30, 'duration': 10.0, 'size': [1920, 1080], 'idle': {'amount': 0.1}, 'follow': 0,
        'camera': CAM, 'look': LOOK_CLIP, 'keys': keys}
path = os.path.join(HERE, 'tut_clip.json')
dump(path, clip)
print(run([H, 'check', path])[1][-2500:])
