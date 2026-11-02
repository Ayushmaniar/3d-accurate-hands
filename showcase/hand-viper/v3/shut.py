"""The jaw shut: from the open head (snake.py), the index fingers curl down onto the thumbs."""
import json, sys
from hk3 import *
from snake import *

if __name__ == '__main__':
    h = json.load(open(os.path.join(OUT, 'snake_hands.json')))
    for s in S:
        h['me.' + s]['index'].update({'mcp_flex': 25, 'pip_flex': 95, 'dip_flex': 35})
    pair = sys.argv[1] if len(sys.argv) > 1 else 'same'
    other = {'right': 'left', 'left': 'right'}
    goals = [{'touch': ['me.right.index.pad', 'me.left.index.nail'], 'vary': IV},
             {'touch': ['me.right.thumb.side_dorsal', 'me.left.thumb.side_dorsal'], 'vary': TV}]
    for s in S:
        t = s if pair == 'same' else other[s]
        goals.append({'touch': ['me.%s.index.tip' % s, 'me.%s.thumb.distal' % t], 'vary': IV + TV})
    h = solve_head(h, goals, 'shut_' + pair)
    for s in S:
        print(s, {k: round(v) for k, v in h['me.' + s]['index'].items()}, {k: round(v) for k, v in h['me.' + s]['thumb'].items()})
    dump(os.path.join(OUT, 'shut_%s_hands.json' % pair), h)
    rows('shut_' + pair, h)
