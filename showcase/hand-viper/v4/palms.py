"""The head as the top-down tutorial (youtube ef1n1x2WlZ0, 2.5 s and 5.4 s) shows it: both palms
cupped toward the viewer, each turned about 45 degrees toward the other, the wrists apart in a V.
The index fingers arch over the top, tips meeting (the upper jaw); middle, ring and little fingers
fold into the mouth, interlaced (their tips show inside, like a tongue); the thumbs stand side by
side at the bottom of the mouth (the lower jaw). Shut: the index tips come down onto the thumbs."""
import sys
from sweep import *

FRONT = {'position': [0.0, 0.09, 0.40], 'target': [0.0, 0.08, 0.0], 'fov': 30}


def palms(a=12, c=-45, half=0.045, index=(40, 70, 30), fold=(60, 90, 50)):
    h = oriented(a, 0, c, half)
    for side in h:
        h[side]['index'] = {'mcp_abd': 0, 'mcp_flex': index[0], 'pip_flex': index[1], 'dip_flex': index[2]}
        for d in ('middle', 'ring', 'pinky'):
            h[side][d] = {'mcp_abd': 0, 'mcp_flex': fold[0], 'pip_flex': fold[1], 'dip_flex': fold[2]}
        h[side]['thumb'] = {'cmc_flex': 25, 'cmc_abd': 40, 'cmc_rot': 30, 'mcp_flex': 10, 'ip_flex': 10}
    return h


ARCH = {'touch': ['right.index.tip', 'left.index.tip'], 'vary': ['right.index.*', 'left.index.*']}
JAW = {'touch': ['right.thumb.tip', 'left.thumb.tip'], 'vary': ['right.thumb.*', 'left.thumb.*']}
CLEAR = {'vary': ['%s.%s.*' % (s, d) for s in ('right', 'left') for d in ('middle', 'ring', 'pinky')]}

if __name__ == '__main__':
    for a, c, half in ((12, -45, 0.045), (15, -55, 0.05), (10, -35, 0.04)):
        n = 'pm_%d_%d_%d' % (a, -c, half * 1000)
        code, text, sol = solve(n, {'version': 1, 'hands': palms(a, c, half), 'goals': [ARCH, JAW, CLEAR]})
        print(n, [l.strip()[:90] for l in text.splitlines() if 'MISS' in l or 'penetr' in l])
        still(n, sol, cam=FRONT, size=380)
