"""A longer snout: index fingers arch forward less curled, their tips down onto the thumb tips."""
import json, sys
from hk3 import *
from snake import *
import clip as C

if __name__ == '__main__':
    tag, mcp, pip, dip = sys.argv[1], *[float(a) for a in sys.argv[2:5]]
    h = json.load(open(os.path.join(OUT, 'snake_hands.json')))
    for s in S:
        h['me.' + s]['index'].update({'mcp_flex': mcp, 'pip_flex': pip, 'dip_flex': dip})
    other = {'right': 'left', 'left': 'right'}
    goals = [{'touch': ['me.right.index.pad', 'me.left.index.nail'], 'vary': IV},
             {'touch': ['me.right.thumb.side_dorsal', 'me.left.thumb.side_dorsal'], 'vary': TV}]
    for s in S:
        goals.append({'touch': ['me.%s.index.tip' % s, 'me.%s.thumb.pad' % other[s]], 'vary': IV + TV})
    h = solve_head(h, goals, 'shut2_' + tag)
    for s in S:
        print(s, {k: round(v) for k, v in h['me.' + s]['index'].items()}, {k: round(v) for k, v in h['me.' + s]['thumb'].items()})
    dump(os.path.join(OUT, 'shut2_%s_hands.json' % tag), h)
    cam3 = {'position': [0.29, 0.15, 0.33], 'target': [0.015, 0.07, 0.0], 'fov': 30}
    rows('shut2_' + tag, h, (('side', SIDE_V), ('3q', TQ_V), ('clip', cam3)))
