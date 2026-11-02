"""Which hand orientation gives the tutorial's head? Fist-like fingers, the hands at +-x, the
fingers up and in by `a`, toward the viewer by `b`, the palm rolled about the fingers by `c`
(0: palms toward each other, positive: toward the viewer). Rendered small from the front."""
import math, sys
from head import *

FIST = {'middle': (60, 95, 50), 'ring': (60, 95, 50), 'pinky': (60, 95, 50), 'index': (70, 85, 30)}


def rot(v, axis, deg):
    a = math.radians(deg)
    k = [x / math.sqrt(sum(y * y for y in axis)) for x in axis]
    dot = sum(v[i] * k[i] for i in range(3))
    cr = [k[1] * v[2] - k[2] * v[1], k[2] * v[0] - k[0] * v[2], k[0] * v[1] - k[1] * v[0]]
    return [v[i] * math.cos(a) + cr[i] * math.sin(a) + k[i] * dot * (1 - math.cos(a)) for i in range(3)]


def oriented(a, b, c, half=0.04):
    h = {}
    for side, s in (('right', 1), ('left', -1)):
        f = rot([0, 1, 0], [0, 0, 1], s * a)          # lean in toward the middle
        f = rot(f, [1, 0, 0], -b)                     # toward the viewer (+z)
        p = rot([-s, 0, 0], f, -s * c)                # roll the palm about the fingers
        h[side] = {'place': {'wrist': [s * half, 0.0, 0.0], 'palm_facing': [round(x, 4) for x in p], 'fingers_toward': [round(x, 4) for x in f],
                             'forearm_toward': [s * 0.2, -1, -0.4]},
                   'thumb': {'cmc_flex': 20, 'cmc_abd': 30, 'cmc_rot': 30, 'mcp_flex': 20, 'ip_flex': 20}}
        for d, (m, pp, dd) in FIST.items():
            h[side][d] = {'mcp_abd': 0, 'mcp_flex': m, 'pip_flex': pp, 'dip_flex': dd}
    return h


CAM = {'position': [0.0, 0.11, 0.42], 'target': [0.0, 0.07, 0.0], 'fov': 30}
if __name__ == '__main__':
    names = []
    for b in (0, 35):
        for c in (-60, -30, 0, 30, 60):
            n = 'sw_b%d_c%d' % (b, c)
            still(n, {'version': 1, 'hands': oriented(20, b, c)}, cam=CAM, size=300)
            names.append(n)
    print(' '.join(names))
