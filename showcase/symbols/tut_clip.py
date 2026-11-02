"""Finger tutting: two geometric figures, each solved with touches, snapped from one to the
other as tutting does.
  frame    the camera frame: each index tip on the other hand's thumb tip, a rectangle
  diamond  index tips together above, thumb tips together below, the fingers straight"""
import copy
from sym import *
import tut

PLACES = ['right.place.position', 'left.place.position']


def figure(name, joints, goals, lean=0.0):
    """Solved from tut's hands with these joint values, each hand's fingers leaning `lean` (of
    up) toward the other hand."""
    h = tut.hands()
    for s in ('right', 'left'):
        x = 1 if s == 'right' else -1
        h[s]['place']['fingers_toward'] = [x * lean, 1, 0]
        for d, v in joints.items():
            h[s][d] = dict(v)
    code, text, sol = solve('tut_' + name, {'version': 1, 'hands': h, 'camera': tut.CAM, 'goals': goals})
    print(name, [l.strip()[:100] for l in text.splitlines() if 'MISS' in l or 'penetrations' in l])
    return sol['hands']


frame = figure('frame', {'index': {'mcp_abd': 0, 'mcp_flex': 0, 'pip_flex': 0, 'dip_flex': 0},
                         'thumb': {'cmc_flex': -30, 'cmc_abd': 0, 'cmc_rot': 0, 'mcp_flex': 0, 'ip_flex': 0}},
               [{'touch': ['right.index.tip', 'left.thumb.tip'], 'vary': ['right.index.*', 'left.thumb.*'] + PLACES},
                {'touch': ['left.index.tip', 'right.thumb.tip'], 'vary': ['left.index.*', 'right.thumb.*', 'right.wrist', 'left.wrist']}])
diamond = figure('diamond', {'index': {'mcp_abd': 0, 'mcp_flex': 10, 'pip_flex': 0, 'dip_flex': 0},
                             'thumb': {'cmc_flex': -20, 'cmc_abd': 10, 'cmc_rot': 0, 'mcp_flex': 0, 'ip_flex': 0}},
                 [{'touch': ['right.index.tip', 'left.index.tip'], 'vary': ['right.index.*', 'left.index.*'] + PLACES},
                  {'touch': ['right.thumb.tip', 'left.thumb.tip'], 'vary': ['right.thumb.*', 'left.thumb.*', 'right.wrist', 'left.wrist']}], lean=0.5)


def at(hands):
    return {'me.' + s: copy.deepcopy(h) for s, h in hands.items()}


keys, t = [{'t': 0, 'hands': at(diamond)}], 0.0
for shape in (frame, diamond, frame, diamond):
    # Each figure holds, then snaps to the next in 0.3 s.
    t += 0.9
    keys.append({'t': round(t, 2), 'hands': at(shape), 'move': 0.3, 'ease': 'snappy'})
clip = {'version': 1, 'fps': 30, 'duration': round(t + 0.6, 2), 'idle': {'amount': 0.08}, 'camera': dict(tut.CAM, fov=38),
        'look': dict(LOOK, motion_blur=3), 'keys': keys}
path = os.path.join(HERE, 'tut_clip.json')
dump(path, clip)
print(run([H, 'check', path])[1][-1200:])
