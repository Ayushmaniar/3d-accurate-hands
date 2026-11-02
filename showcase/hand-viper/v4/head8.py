"""A plain interlocked-finger clasp (palms close, the grip closing middle, ring and little
fingers over the other hand's back) seen from the thumb side, the index fingers curled across the
front, tips meeting (the snout), each thumb pad up against its index's middle segment (the jaw)."""
import sys
from head import *

CAM8 = {'position': [0.0, 0.17, 0.36], 'target': [0.0, 0.09, 0.0], 'fov': 28}
GRIP = {'grip': ['right', 'left'], 'style': 'interlaced', 'vary': ['%s.%s.*' % (s, d) for s in ('right', 'left') for d in ('middle', 'ring', 'pinky')]}


def clasp(half=0.03, dz=0.006, lean=0.3, index=(40, 90, 30)):
    h = hands(half, dz)
    for side, s in (('right', 1), ('left', -1)):
        h[side]['place']['fingers_toward'] = [0, 1, lean]
        h[side]['index'] = {'mcp_abd': -5, 'mcp_roll': 0, 'mcp_flex': index[0], 'pip_flex': index[1], 'dip_flex': index[2]}
        h[side]['thumb'] = {'cmc_flex': 30, 'cmc_abd': 10, 'cmc_rot': 25, 'mcp_flex': 30, 'ip_flex': 25}
    return h


def goals():
    return [GRIP, {'touch': ['right.index.tip', 'left.index.tip'], 'vary': ['right.index.*', 'left.index.*']}] + \
           [{'touch': ['%s.thumb.pad' % s, '%s.index.middle' % s], 'vary': ['%s.thumb.*' % s]} for s in ('right', 'left')]


if __name__ == '__main__':
    names = []
    for half, lean, idx in ((0.03, 0.3, (40, 90, 30)), (0.034, 0.6, (30, 95, 35)), (0.03, 0.9, (50, 90, 30))):
        n = 'h8_%d_%d_%d' % (half * 1000, lean * 10, idx[0])
        code, text, sol = solve(n, {'version': 1, 'hands': clasp(half, lean=lean, index=idx), 'goals': goals()})
        print(n, [l.strip()[:90] for l in text.splitlines() if 'MISS' in l or 'penetr' in l])
        still(n, sol, cam=CAM8, size=360)
        names.append(n)
    print(' '.join(names))
