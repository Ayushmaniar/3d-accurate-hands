"""The hand viper facing the camera, after the tutorial (youtube uroglg7zo8E), 8 s:
  0.0  both hands up, palms to the viewer, fingers spread ("hold your hands out")
  1.6  the hands turn in toward each other and the fingers interweave (the steps, compressed)
  3.2  the interlaced fingers fold over: the dome; index fingers and thumbs still out
  4.4  index fingers curl over into the arch, thumbs come up underneath: the head, mouth shut
  5.4 / 5.9  the mouth opens and shuts (thumbs drop away from the index fingers and back)
  6.7  rears back, mouth wide; 7.0 strikes forward and snaps shut; holds."""
import copy
import os
os.environ.setdefault('DOME9', '40,100,55')
from head9 import *

HALF, LEAN = 0.045, 0.8
CAM = {'position': [0.0, 0.10, 0.52], 'target': [0.0, 0.07, 0.0], 'fov': 33}
HEAD_CAM = {'position': [0.0, 0.11, 0.42], 'target': [0.0, 0.08, 0.0], 'fov': 28}
LOOK_CLIP = {'style': 'realistic', 'skin': 'medium', 'motion_blur': 3, 'arm': {'sleeve': '#262a33', 'length': 0.12}}

shut = solve('clip_shut', {'version': 1, 'hands': cupped(HALF, LEAN), 'goals': [ARCH, SIDE] + SHUT + [CLEAR]})[2]['hands']


def opened(hands, amount=1.0):
    out = copy.deepcopy(hands)
    for s in out:
        t = out[s]['thumb']
        t['cmc_flex'] = round(max(-30, t['cmc_flex'] - 25 * amount), 1)
        t['cmc_abd'] = round(max(-10, t['cmc_abd'] - 15 * amount), 1)
        ix = out[s]['index']
        ix['mcp_flex'] = round(ix['mcp_flex'] - 8 * amount, 1)
    return out


def moved(hands, dy=0.0, dz=0.0):
    out = copy.deepcopy(hands)
    for s in out:
        w = out[s]['place']['wrist']
        out[s]['place']['wrist'] = [w[0], round(w[1] + dy, 4), round(w[2] + dz, 4)]
    return out


def start():
    """Palms to the viewer, fingers up and spread, the hands apart."""
    out = {}
    for side, s in (('right', 1), ('left', -1)):
        out[side] = {'place': {'wrist': [s * 0.075, -0.01, 0.0], 'palm_facing': [0, 0, 1], 'fingers_toward': [s * 0.15, 1, 0],
                               'forearm_toward': [s * 0.3, -1, -0.4]},
                     'thumb': {'cmc_flex': -15, 'cmc_abd': 0, 'cmc_rot': 0, 'mcp_flex': 0, 'ip_flex': 0}}
        for d, a in (('index', 8), ('middle', 2), ('ring', 6), ('pinky', 12)):
            out[side][d] = {'mcp_abd': a, 'mcp_flex': 5, 'pip_flex': 5, 'dip_flex': 3}
    return out


def blend(a, b, f):
    """Joint values and placement directions part way from a to b (for the in-between steps)."""
    out = copy.deepcopy(b)
    for s in out:
        for d in ('thumb', 'index', 'middle', 'ring', 'pinky'):
            for k in out[s][d]:
                out[s][d][k] = round(a[s][d].get(k, 0) + (b[s][d][k] - a[s][d].get(k, 0)) * f, 1)
        for k in ('wrist', 'palm_facing', 'fingers_toward', 'forearm_toward'):
            out[s]['place'][k] = [round(a[s]['place'][k][i] + (b[s]['place'][k][i] - a[s]['place'][k][i]) * f, 4) for i in range(3)]
    return out


def me(h):
    return {'me.' + s: v for s, v in h.items()}


s0 = start()
# Interweave: the hands half turned in, the pinky-side fingers half closed; then the dome with
# the index fingers still straight and the thumbs out.
weave = blend(s0, shut, 0.75)
for s in weave:
    weave[s]['index'] = {'mcp_abd': 0, 'mcp_flex': 5, 'pip_flex': 5, 'dip_flex': 3}
    weave[s]['thumb'] = dict(s0[s]['thumb'])
dome = copy.deepcopy(shut)
for s in dome:
    dome[s]['index'] = {'mcp_abd': 0, 'mcp_flex': 0, 'pip_flex': 10, 'dip_flex': 5}
    # The thumbs already part way up, each in its own place, so they rise into the jaw side by side.
    dome[s]['thumb'] = {k: round(v - {'cmc_flex': 15, 'cmc_abd': 10}.get(k, 0), 1) for k, v in shut[s]['thumb'].items()}

keys = [
    {'t': 0.0, 'hands': me(s0)},
    {'t': 1.6, 'hands': me(weave), 'move': 1.2, 'ease': 'smooth'},
    {'t': 3.2, 'hands': me(dome), 'move': 1.1, 'ease': 'smooth'},
    {'t': 3.2, 'camera': CAM},
    {'t': 4.4, 'camera': HEAD_CAM, 'move': 1.2, 'ease': 'smooth'},
    {'t': 4.4, 'hands': me(shut), 'move': 0.9, 'ease': 'smooth'},
    {'t': 5.4, 'hands': me(opened(shut, 0.7)), 'move': 0.3, 'ease': 'snappy'},
    {'t': 5.9, 'hands': me(shut), 'move': 0.25, 'ease': 'snappy'},
    {'t': 6.7, 'hands': me(moved(opened(shut, 1.0), 0.01, -0.03)), 'move': 0.5, 'ease': 'smooth'},
    {'t': 7.0, 'hands': me(moved(shut, -0.005, 0.04)), 'move': 0.14, 'ease': 'snappy'},
    {'t': 7.6, 'hands': me(shut), 'move': 0.5},
]
clip = {'version': 1, 'fps': 30, 'duration': 8.0, 'size': [1920, 1080], 'idle': {'amount': 0.1}, 'collide': False,
        'camera': CAM, 'look': LOOK_CLIP, 'keys': keys}
path = os.path.join(HERE, 'clip.json')
dump(path, clip)
print(run([H, 'check', path])[1][-2500:])
