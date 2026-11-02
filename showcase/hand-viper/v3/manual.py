"""Hand-set head joints on the upright clasp, rendered from the side, the front and 3/4.
usage: manual.py NAME '{"right": {"index": {...}, "thumb": {...}}, "left": {...}}'"""
import json, sys
from hk3 import *
from vgrip import row
import math


def lean(h, deg):
    """Tips the whole knot forward (fingers toward +x) about the z axis through the wrists."""
    c, s = math.cos(math.radians(deg)), math.sin(math.radians(deg))
    r = lambda v: [v[0] * c + v[1] * s, -v[0] * s + v[1] * c, v[2]]
    h = copy.deepcopy(h)
    for hand in h.values():
        pl = hand['place']
        for k in ('wrist', 'palm_facing', 'fingers_toward', 'forearm_toward'):
            pl[k] = r(pl[k])
    return h

if __name__ == '__main__':
    name, over = sys.argv[1], json.loads(sys.argv[2])
    tilt = float(sys.argv[3]) if len(sys.argv) > 3 else 0.0
    h = json.load(open(os.path.join(OUT, 'upright_hands.json')))
    for side, digits in over.items():
        for d, vals in digits.items():
            h['me.' + side][d] = dict(h['me.' + side].get(d, {}), **vals) if d != 'place' else vals
    h = lean(h, tilt)
    q = query(pose(h))
    print('\n'.join(l for l in q.splitlines() if 'index' in l or 'thumb' in l or 'violation' in l)[:1500])
    dump(os.path.join(OUT, name + '.json'), h)
    row(name, h)
