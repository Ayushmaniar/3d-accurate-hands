"""Solve the six step poses (and the strike) of the hand viper with touch goals.
Writes steps_sol.json: {"1": hands, ..., "7": hands}. Args: which steps to preview."""
import json, sys, copy, math
from hk import *

def R(w, pf, ft, fa): return {'wrist': w, 'palm_facing': pf, 'fingers_toward': ft, 'forearm_toward': fa}
def mirror_place(pl): return {k: [-v[0], v[1], v[2]] for k, v in pl.items()}
FA = [-0.5, -0.7, -0.5]
def V(tilt, wx, fa=FA): return R([wx, 0.0, 0.0], [1, tilt, 0], [-tilt, 1, 0], fa)
def pair(right, left_overrides=None):
    left = copy.deepcopy(right); left['place'] = mirror_place(right['place'])
    for k, v in (left_overrides or {}).items(): left[k].update(v)
    return {'right': right, 'left': left}

which = sys.argv[1:]
S = {}
def step(n, hands, goals):
    c, t, s = solve('st%d' % n, hands, goals)
    q = query('st%d_sol.json' % n)
    miss = [l.strip() for l in q.splitlines() if 'MISS' in l or 'outside' in l]
    pen = q.split('penetrations')[1].splitlines()[0]
    print(n, 'exit', c, t.splitlines()[0], '| pen', pen, '|', miss)
    S[n] = s['hands']

# 1: palms pressed together, fingers up
r1 = {'place': V(-0.1, -0.024),
  'thumb': {'cmc_flex': 6, 'cmc_abd': -10, 'cmc_rot': -5, 'mcp_flex': -2, 'ip_flex': -4},
  'index': {'mcp_abd': -2, 'mcp_flex': 0, 'pip_flex': 4},
  'middle': {'mcp_abd': 0, 'mcp_flex': 0, 'pip_flex': 4},
  'ring': {'mcp_abd': 1, 'mcp_flex': 0, 'pip_flex': 4},
  'pinky': {'mcp_abd': 2, 'mcp_flex': 0, 'pip_flex': 4}}
G1 = [{'touch': ['right.thumb.distal', 'left.thumb.distal'], 'vary': ['right.thumb.*', 'left.thumb.*']},
      {'touch': ['right.index.distal', 'left.index.distal'], 'vary': ['right.index.mcp_flex', 'left.index.mcp_flex', 'right.index.pip_flex', 'left.index.pip_flex']},
      {'touch': ['right.middle.distal', 'left.middle.distal'], 'vary': ['right.middle.mcp_flex', 'left.middle.mcp_flex', 'right.middle.pip_flex', 'left.middle.pip_flex']},
      {'touch': ['right.ring.distal', 'left.ring.distal'], 'vary': ['right.ring.mcp_flex', 'left.ring.mcp_flex', 'right.ring.pip_flex', 'left.ring.pip_flex']},
      {'touch': ['right.pinky.distal', 'left.pinky.distal'], 'vary': ['right.pinky.mcp_flex', 'left.pinky.mcp_flex', 'right.pinky.pip_flex', 'left.pinky.pip_flex']}]
step(1, pair(r1), G1)

# 2: ring fingers cross down over each other, hands opened into a V
right = {'place': V(0.35, -0.03),
  'thumb': {'cmc_flex': 0, 'cmc_abd': 5, 'cmc_rot': 0, 'mcp_flex': 0, 'ip_flex': 0},
  'index': {'mcp_abd': 5, 'mcp_flex': 5, 'pip_flex': 8},
  'middle': {'mcp_abd': 0, 'mcp_flex': 8, 'pip_flex': 8},
  'ring': {'mcp_abd': -6, 'mcp_flex': 70, 'pip_flex': 25},
  'pinky': {'mcp_abd': 8, 'mcp_flex': 35, 'pip_flex': 30}}
G2 = [{'touch': ['right.ring.middle', 'left.ring.middle'], 'vary': ['right.ring.*', 'left.ring.*']}]
step(2, pair(right, {'ring': {'mcp_abd': 12}}), G2)

# 3: middle fingers on top
h = copy.deepcopy(S[2])
for side, ab in (('right', -4), ('left', 6)):
    h[side]['middle'] = {'mcp_abd': ab, 'mcp_flex': 62, 'pip_flex': 30}
G3 = G2 + [{'touch': ['right.middle.middle', 'left.middle.middle'], 'vary': ['right.middle.*', 'left.middle.*']},
           {'touch': ['right.middle.proximal', 'left.ring.middle'], 'vary': ['right.middle.*']},
           {'touch': ['left.middle.proximal', 'right.ring.middle'], 'vary': ['left.middle.*']}]
step(3, h, G3)

# 4: index fingers curve forward, tips meet: the front of the face
h = copy.deepcopy(S[3])
for side in ('right', 'left'):
    h[side]['index'] = {'mcp_abd': 12, 'mcp_flex': 45, 'pip_flex': 50}
G4 = G3[:2] + [{'touch': ['right.index.tip', 'left.index.tip'], 'vary': ['right.index.*', 'left.index.*']}]
step(4, h, G4)

# 5: thumbs underneath, tips together: the lower jaw
h = copy.deepcopy(S[4])
for side in ('right', 'left'):
    h[side]['thumb'] = {'cmc_flex': -25, 'cmc_abd': 25, 'cmc_rot': 10, 'mcp_flex': 25, 'ip_flex': 25}
G5 = G4 + [{'touch': ['right.thumb.tip', 'left.thumb.tip'], 'vary': ['right.thumb.*', 'left.thumb.*']}]
step(5, h, G5)

# 6: squeeze into a triangle: hands pressed closer, jaw shut under the snout
h = copy.deepcopy(S[5])
h['right']['place'] = V(0.25, -0.023); h['left']['place'] = mirror_place(h['right']['place'])
for side in ('right', 'left'):
    h[side]['thumb'].update({'cmc_abd': 15, 'mcp_flex': 15, 'ip_flex': 25})
    h[side]['pinky'] = {'mcp_abd': 4, 'mcp_flex': 45, 'pip_flex': 45}
G6 = G5 + [{'touch': ['right.thumb.nail', 'right.index.pad'], 'gap_mm': 14, 'vary': ['right.thumb.*']},
           {'touch': ['left.thumb.nail', 'left.index.pad'], 'gap_mm': 14, 'vary': ['left.thumb.*']},
           {'vary': ['right.ring.*', 'left.ring.*', 'right.pinky.*', 'left.pinky.*']}]
step(6, h, G6)

json.dump({str(k): v for k, v in S.items()}, open('steps_sol.json', 'w'), indent=1)
for k in S:
    if str(k) in which: print(preview('st%d' % k, S[k])[0])
