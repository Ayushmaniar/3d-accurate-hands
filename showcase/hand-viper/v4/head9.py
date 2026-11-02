"""The head as the tutorial's mouth-open close-up (48.3 s) shows it: two cupped hands, palms
toward each other and the thumb sides to the viewer; the mouth is the hollow between the palms.
The index fingers curl over the top front into an arch, tips meeting (the upper jaw); the thumbs
stand side by side at the bottom middle, tips up (the lower jaw); middle, ring and little fingers
interlaced behind and above (the dome, the eyes on top). Shut: the thumb tips meet the index
tips. Open: the thumbs drop away."""
import sys
from head2 import *

CAM9 = {'position': [0.0, 0.07, 0.34], 'target': [0.0, 0.085, 0.0], 'fov': 26}
THUMB_UP = {'cmc_flex': 20, 'cmc_abd': 20, 'cmc_rot': 20, 'mcp_flex': 10, 'ip_flex': 10}
SIDE = {'touch': ['right.thumb.side_palmar', 'left.thumb.side_palmar'], 'vary': ['right.thumb.*', 'left.thumb.*']}


def cupped(half=0.05, lean=0.5):
    DOME.update(zip(('mcp_flex', 'pip_flex', 'dip_flex'), (float(x) for x in os.environ.get('DOME9', '15,95,45').split(','))))
    h = hands2(half, 0.006, lean)
    for s in h:
        h[s]['thumb'] = dict(THUMB_UP)
    return h


ARCH = {'touch': ['right.index.tip', 'left.index.tip'], 'vary': V_SNOUT}
CLEAR = {'vary': ['%s.%s.*' % (s, d) for s in ('right', 'left') for d in ('middle', 'ring', 'pinky')]}
SHUT = [{'touch': ['%s.thumb.tip' % s, '%s.index.distal' % s], 'vary': ['%s.thumb.*' % s]} for s in ('right', 'left')]

if __name__ == '__main__':
    half, lean = (float(sys.argv[1]), float(sys.argv[2])) if len(sys.argv) > 2 else (0.05, 0.5)
    tag = '%d_%d' % (half * 1000, lean * 10)
    code, text, shut = solve('h9s_' + tag, {'version': 1, 'hands': cupped(half, lean), 'goals': [ARCH, SIDE] + SHUT + [CLEAR]})
    print('shut', [l.strip()[:90] for l in text.splitlines() if 'MISS' in l or 'penetr' in l])
    still('h9s_' + tag, shut, cam=CAM9, size=380)
    opened = json.loads(json.dumps(shut))
    for s in opened['hands']:
        t = opened['hands'][s]['thumb']
        t['cmc_flex'] = max(-30, t['cmc_flex'] - 25)
        t['cmc_abd'] = max(-10, t['cmc_abd'] - 15)
    still('h9o_' + tag, opened, cam=CAM9, size=380)
