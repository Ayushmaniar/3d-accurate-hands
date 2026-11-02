"""The snake's head on the upright clasp: index fingers curled forward over the front, one on
the other (the snout); thumbs forward underneath, side by side (the jaw)."""
import json, sys
from hk3 import *
import upright
from upright import SIDE_UP, FACE

THREE_Q = {'position': [0.30, 0.16, 0.30], 'target': [0.02, 0.07, 0.0], 'fov': 30}
SIDES = ('right', 'left')
INDEX = {'mcp_abd': 15, 'mcp_flex': 45, 'pip_flex': 70, 'dip_flex': 40}
THUMB = {'cmc_flex': -20, 'cmc_abd': 10, 'cmc_rot': 0, 'mcp_flex': 5, 'ip_flex': 10}


def report(tag, code, text, h):
    q = query(pose(h))
    pens = q.split('penetrations')[1].split('contacts')[0].strip().splitlines()[1:]
    print(tag, code, text.splitlines()[0], '| pen', len(pens), [p.strip() for p in pens[:4]])


def head(base):
    h = copy.deepcopy(base)
    for s in SIDES:
        h['me.' + s]['index'] = dict(INDEX)
        h['me.' + s]['thumb'] = dict(THUMB)
    goals = [{'touch': ['me.right.index.pad', 'me.left.index.nail'], 'vary': ['me.right.index.*', 'me.left.index.*']},
             {'touch': ['me.right.thumb.side_palmar', 'me.left.thumb.side_palmar'], 'vary': ['me.right.thumb.*', 'me.left.thumb.*']}]
    code, text, s = solve('head', pose(h, goals))
    report('head', code, text, s['hands'])
    return s['hands']


if __name__ == '__main__':
    base = json.load(open(os.path.join(OUT, 'upright_hands.json')))
    h = head(base)
    for s in SIDES:
        print(s, 'index', h['me.' + s]['index'], 'thumb', h['me.' + s]['thumb'])
    dump(os.path.join(OUT, 'head_hands.json'), h)
    still('head_side', h, SIDE_UP)
    still('head_face', h, FACE)
    still('head_3q', h, THREE_Q)
