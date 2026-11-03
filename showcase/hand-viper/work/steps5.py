"""Pass two, final: the viper built on a real interlaced clasp (gm_sol.json, from grip2.py:
Handkit's interlaced grip between one person's own hands, mirrored, left hand 1 cm toward
the pinkies so the fingers alternate). Knot frame: palms +-X, fingers +Y, thumb side -Z
(where the head points). orient_profile() stands it up for the film, seen from the back of
the left hand with the head pointing screen right.
Writes steps5_sol.json {"1".."6", "7" (strike, jaw open)}. Args: steps to preview."""
import json, sys, copy, os
from hk import *
from orient import orient_profile, transform

HANDS = ('right', 'left')
G = json.load(open(os.path.join(WORK, 'gm_sol.json')))['hands']
S1 = json.load(open(os.path.join(WORK, 'st1_sol.json')))['hands']
FAW = {'left': [0.72, -0.67, -0.15], 'right': [0.72, -0.67, 0.15]}
PROF = {'position': [-0.09, 0.05, -0.36], 'target': [-0.01, 0.0, 0.0], 'fov': 30}
TILT = -25
REST_T = {'cmc_flex': 12, 'cmc_abd': -8, 'cmc_rot': 0, 'mcp_flex': 10, 'ip_flex': 10}
OUT = {'mcp_abd': 0, 'mcp_flex': 4, 'pip_flex': 6, 'dip_flex': 4}
INDEX = {'mcp_abd': 20, 'mcp_flex': 40, 'pip_flex': 65, 'dip_flex': 40}
THUMB = {'cmc_flex': -28, 'cmc_abd': 2, 'cmc_rot': 5, 'mcp_flex': 0, 'ip_flex': 5}
SNOUT = {'touch': ['right.index.pad', 'left.index.nail'], 'vary': ['right.index.*', 'left.index.*']}
JAW = {'touch': ['right.thumb.pad', 'left.thumb.nail'], 'vary': ['right.thumb.*', 'left.thumb.*']}   # stacked, side by side


def clear(*digits):
    return {'vary': ['%s.%s.*' % (s, d) for d in digits for s in HANDS]}


def world(hands, tilt=TILT):
    return transform(hands, orient_profile(tilt), forearms=FAW)


def step(n, hands, goals):
    c, t, s = solve('v%d' % n, hands, goals)
    q = query('v%d_sol.json' % n)
    miss = [l.strip()[:100] for l in q.splitlines() if 'MISS' in l or 'outside' in l]
    pens = q.split('penetrations')[1].split('contacts')[0].strip().splitlines()[1:]
    print(n, t.splitlines()[0], '| pen', len(pens), [p.strip() for p in pens[:3]], '|', miss)
    return s['hands']


def build():
    S = {1: S1}
    h = copy.deepcopy(G)                       # 2: ring fingers cross and hook over
    for s in HANDS:
        for d in ('index', 'middle'):
            h[s][d] = dict(OUT)
        h[s]['pinky'] = dict(G[s]['pinky'])
        h[s]['thumb'] = dict(REST_T)
    S[2] = step(2, h, [clear('index', 'middle', 'thumb')])
    h = copy.deepcopy(S[2])                    # 3: middle fingers over them
    for s in HANDS:
        h[s]['middle'] = dict(G[s]['middle'])
    S[3] = step(3, h, [clear('index', 'thumb')])
    h = copy.deepcopy(S[3])                    # 4: index fingers curl forward: the face
    for s in HANDS:
        h[s]['index'] = dict(INDEX)
    S[4] = step(4, h, [SNOUT, clear('thumb')])
    h = copy.deepcopy(S[4])                    # 5: thumbs side by side underneath: the jaw
    for s in HANDS:
        h[s]['thumb'] = dict(THUMB)
    S[5] = step(5, h, [SNOUT, JAW])
    h = copy.deepcopy(S[5])                    # 6: squeeze: the jaw shuts under the snout
    shut = [{'touch': ['left.index.tip', 'left.thumb.distal'], 'vary': ['left.index.*', 'left.thumb.*']},
            {'touch': ['right.index.tip', 'right.thumb.distal'], 'vary': ['right.index.*', 'right.thumb.*']}]
    S[6] = step(6, h, [JAW] + shut + [clear('ring', 'middle', 'pinky')])
    h = copy.deepcopy(S[6])                    # 7: strike: the jaw drops wide open
    for s in HANDS:
        h[s]['thumb'].update({'cmc_flex': -30, 'mcp_flex': -10, 'ip_flex': -15})
        h[s]['index'] = dict(h[s]['index']); h[s]['index']['mcp_flex'] = max(-28, h[s]['index']['mcp_flex'] - 12)
        h[s]['index']['pip_flex'] = max(20, h[s]['index']['pip_flex'] - 20)
    S[7] = step(7, h, [JAW, clear('index')])
    return S


if __name__ == '__main__':
    S = build()
    json.dump({str(k): v for k, v in S.items()}, open(os.path.join(WORK, 'steps5_sol.json'), 'w'), indent=1)
    for a in sys.argv[1:]:
        n = int(a)
        preview('v%d' % n, world(S[n]), camera=PROF, width=480)
