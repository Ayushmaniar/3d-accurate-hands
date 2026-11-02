"""From the orientation sweep: palms rolled toward the viewer, the fingers up and in. Solved:
middle, ring and little fingers interlaced (the grip), index tips meeting across the front (the
snout), thumb tips side by side pressed up under the index fingers (the jaw, shut)."""
import sys
from sweep import *

A, B, C, HALF = [float(x) for x in (sys.argv[1:5] if len(sys.argv) > 4 else (20, 10, -50, 0.045))]
GRIP = {'grip': ['right', 'left'], 'style': 'interlaced', 'vary': ['%s.%s.*' % (s, d) for s in ('right', 'left') for d in ('middle', 'ring', 'pinky')]}
SNOUT = {'touch': ['right.index.tip', 'left.index.tip'], 'vary': ['right.index.*', 'left.index.*']}
THUMBS = {'touch': ['right.thumb.tip', 'left.thumb.tip'], 'vary': ['right.thumb.*', 'left.thumb.*']}
JAW = [{'touch': ['%s.thumb.pad' % s, '%s.index.distal' % s], 'vary': ['%s.thumb.*' % s]} for s in ('right', 'left')]

if __name__ == '__main__':
    name = 'h5_%d_%d_%d' % (A, B, C)
    pose = {'version': 1, 'hands': oriented(A, B, C, HALF), 'goals': [GRIP, SNOUT, THUMBS] + JAW}
    code, text, sol = solve(name, pose)
    print(name, code, [l.strip()[:100] for l in text.splitlines() if 'MISS' in l or 'penetr' in l])
    still(name, sol, cam=CAM, size=420)
