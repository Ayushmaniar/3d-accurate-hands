"""The hand viper facing the camera (v4b), 8 s, built on the head the two tutorials show (palms
cupped toward the viewer, index arch over the top, thumbs underneath, the other fingers folded in):
  0.0  both hands up, palms to the viewer, fingers spread
  1.6  the hands turn in toward each other, middle, ring and little fingers folding in
  3.2  folded; index fingers and thumbs still out
  4.4  index fingers arch over and the thumbs come up: the head, mouth shut
  5.4 / 5.9  the mouth opens and shuts; 6.7 rears back mouth wide; 7.0 strikes and snaps shut."""
import copy
from mouth import *

CAM = {'position': [0.0, 0.10, 0.52], 'target': [0.0, 0.07, 0.0], 'fov': 33}
HEAD_CAM = {'position': [0.0, 0.10, 0.40], 'target': [0.0, 0.08, 0.0], 'fov': 28}
LOOK_CLIP = {'style': 'realistic', 'skin': 'medium', 'motion_blur': 3, 'arm': {'sleeve': '#262a33', 'length': 0.12}}
DIG = ('thumb', 'index', 'middle', 'ring', 'pinky')

shut = head_shut()


def start():
    out = {}
    # The right hand on the right (as in the head): palm to the viewer, its thumb points out.
    for side, s in (('right', 1), ('left', -1)):
        out[side] = {'place': {'wrist': [s * 0.075, -0.01, 0.0], 'palm_facing': [0, 0, 1], 'fingers_toward': [-s * 0.15, 1, 0],
                               'forearm_toward': [s * 0.3, -1, -0.4]},
                     'thumb': {'cmc_flex': -15, 'cmc_abd': 0, 'cmc_rot': 0, 'mcp_flex': 0, 'ip_flex': 0}}
        for d, a in (('index', 8), ('middle', 2), ('ring', 6), ('pinky', 12)):
            out[side][d] = {'mcp_abd': a, 'mcp_flex': 5, 'pip_flex': 5, 'dip_flex': 3}
    return out


def blend(a, b, f):
    out = copy.deepcopy(b)
    for s in out:
        for d in DIG:
            for k in out[s][d]:
                out[s][d][k] = round(a[s][d].get(k, 0) + (b[s][d][k] - a[s][d].get(k, 0)) * f, 1)
        for k in ('wrist', 'palm_facing', 'fingers_toward', 'forearm_toward'):
            out[s]['place'][k] = [round(a[s]['place'][k][i] + (b[s]['place'][k][i] - a[s]['place'][k][i]) * f, 4) for i in range(3)]
    return out


def moved(hands, dy=0.0, dz=0.0):
    out = copy.deepcopy(hands)
    for s in out:
        w = out[s]['place']['wrist']
        out[s]['place']['wrist'] = [w[0], round(w[1] + dy, 4), round(w[2] + dz, 4)]
    return out


def me(h):
    return {'me.' + s: v for s, v in h.items()}


s0 = start()
fold = copy.deepcopy(shut)
for s in fold:
    fold[s]['index'] = {'mcp_abd': 0, 'mcp_flex': 5, 'pip_flex': 8, 'dip_flex': 4}
    fold[s]['thumb'] = {'cmc_flex': 0, 'cmc_abd': 10, 'cmc_rot': 10, 'mcp_flex': 0, 'ip_flex': 0}
turn = blend(s0, fold, 0.55)

keys = [
    {'t': 0.0, 'hands': me(s0)},
    {'t': 1.6, 'hands': me(turn), 'move': 1.2, 'ease': 'smooth'},
    {'t': 3.2, 'hands': me(fold), 'move': 1.2, 'ease': 'smooth'},
    {'t': 3.2, 'camera': CAM},
    {'t': 4.4, 'camera': HEAD_CAM, 'move': 1.2, 'ease': 'smooth'},
    {'t': 4.4, 'hands': me(shut), 'move': 0.9, 'ease': 'smooth'},
    {'t': 5.4, 'hands': me(head_open(shut, 0.7)), 'move': 0.3, 'ease': 'snappy'},
    {'t': 5.9, 'hands': me(shut), 'move': 0.25, 'ease': 'snappy'},
    {'t': 6.7, 'hands': me(moved(head_open(shut, 1.0), 0.01, -0.035)), 'move': 0.5, 'ease': 'smooth'},
    {'t': 7.0, 'hands': me(moved(shut, -0.005, 0.045)), 'move': 0.14, 'ease': 'snappy'},
    {'t': 7.6, 'hands': me(shut), 'move': 0.5},
]
clip = {'version': 1, 'fps': 30, 'duration': 8.0, 'size': [1920, 1080], 'idle': {'amount': 0.1}, 'collide': False,
        'camera': CAM, 'look': LOOK_CLIP, 'keys': keys}
path = os.path.join(HERE, 'clip.json')
dump(path, clip)
print(run([H, 'check', path])[1][-1800:])
