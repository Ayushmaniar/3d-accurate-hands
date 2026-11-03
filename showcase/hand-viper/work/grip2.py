"""Make gm_sol.json: one person's two hands in Handkit's interlaced grip (every finger curled
over the back of the other hand), placed as a mirror pair with the left hand 1 cm toward the
pinkies so the fingers alternate. Only the fingers are solved; the placement is kept.
Usage: python grip2.py -0.019 0.01   (right wrist x in the knot frame, left hand z offset)"""
import json, copy, sys
from hk import *
from steps5 import world, PROF


def R(w, pf, ft, fa): return {'wrist': w, 'palm_facing': pf, 'fingers_toward': ft, 'forearm_toward': fa}
def mirror_place(pl): return {k: [-v[0], v[1], v[2]] for k, v in pl.items()}


wx = float(sys.argv[1]); dz = float(sys.argv[2])
h = copy.deepcopy(json.load(open('st1_sol.json'))['hands'])
h['right']['place'] = R([wx, 0.0, 0.0], [1, 0, 0], [0, 1, 0], [-0.5, -0.7, -0.5])
pl = mirror_place(h['right']['place']); pl['wrist'][2] += dz; h['left']['place'] = pl
vary = ['%s.%s.*' % (s, d) for s in ('right', 'left') for d in ('thumb', 'index', 'middle', 'ring', 'pinky')]
c, t, s = solve('gm', h, [{'grip': ['right', 'left'], 'style': 'interlaced', 'vary': vary}])
print(c, t.splitlines()[0])
preview('gm_profile', world(s['hands']), camera=PROF, width=560)
