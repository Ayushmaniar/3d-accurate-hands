"""Head with the fingers really interlaced: the hands close (`half` m from the middle) and offset
front to back by `dz` so the fingers alternate; front and top views."""
import sys
from head9 import *


def build(half, dz, lean=0.8, tag='e'):
    h = cupped(half, lean)
    for side, s in (('right', 1), ('left', -1)):
        h[side]['place']['wrist'][2] = s * dz
    code, text, shut = solve(tag, {'version': 1, 'hands': h, 'goals': [ARCH, SIDE] + SHUT + [CLEAR]})
    print(tag, [l.strip()[:90] for l in text.splitlines() if 'MISS' in l or 'penetr' in l])
    still(tag + '_f', shut, cam={'position': [0.0, 0.12, 0.42], 'target': [0.0, 0.08, 0.0], 'fov': 28}, size=360)
    still(tag + '_t', shut, cam={'position': [0.0, 0.34, 0.2], 'target': [0.0, 0.08, 0.03], 'fov': 28}, size=360)
    return shut


if __name__ == '__main__':
    for half, dz in ((0.03, 0.008), (0.025, 0.011)):
        build(half, dz, tag='e_%d_%d' % (half * 1000, dz * 1000))
