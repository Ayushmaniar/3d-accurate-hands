"""Back to h2 (upright fists, palms toward each other, the thumb sides to the viewer; the other
fingers interlaced up and over as the dome; the index fingers curled across the front, tips
meeting): the thumbs tucked, each pressing its index's middle segment from below (the jaw
shut), the fists wider apart, the camera a little above as in the tutorial."""
import sys
from head2 import *

TUCK = {'cmc_flex': 35, 'cmc_abd': 15, 'cmc_rot': 35, 'mcp_flex': 45, 'ip_flex': 45}
CAM6 = {'position': [0.0, 0.17, 0.37], 'target': [0.0, 0.085, 0.0], 'fov': 28}


def hands6(half=0.04, dz=0.006, lean=0.0):
    h = hands2(half, dz, lean)
    for s in h:
        h[s]['thumb'] = dict(TUCK)
    return h


GOALS6 = [{'touch': ['right.index.tip', 'left.index.tip'], 'vary': V_SNOUT},
          {'touch': ['right.thumb.tip', 'right.index.middle'], 'vary': ['right.thumb.*']},
          {'touch': ['left.thumb.tip', 'left.index.middle'], 'vary': ['left.thumb.*']},
          {'vary': ['%s.%s.*' % (s, d) for s in ('right', 'left') for d in ('middle', 'ring', 'pinky')]}]

if __name__ == '__main__':
    half, lean = (float(sys.argv[1]), float(sys.argv[2])) if len(sys.argv) > 2 else (0.04, 0.0)
    name = 'h6_%d_%d' % (half * 1000, lean * 100)
    code, text, sol = solve(name, {'version': 1, 'hands': hands6(half, lean=lean), 'goals': GOALS6})
    print(name, code, [l.strip()[:100] for l in text.splitlines() if 'MISS' in l or 'penetr' in l])
    still(name, sol, cam=CAM6, size=420)
