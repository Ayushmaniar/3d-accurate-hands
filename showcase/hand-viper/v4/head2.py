"""Second pass at the tutorial's head: the interlaced fingers rise and fold over the top (the
dome), the index fingers curl tight across the front below them, tips meeting (the snout),
each thumb tip against the underside of its own index (the jaw, shut)."""
from head import *

import os
DOME = {k: float(v) for k, v in zip(('mcp_flex', 'pip_flex', 'dip_flex'), os.environ.get('DOME', '15,95,45').split(','))}


def hands2(half=0.03, dz=0.006, lean=0.0):
    h = hands(half, dz)
    for side, s in (('right', 1), ('left', -1)):
        h[side]['place']['fingers_toward'] = [0, 1, lean]
        h[side]['index'] = {'mcp_abd': -5, 'mcp_roll': 0, 'mcp_flex': 85, 'pip_flex': 95, 'dip_flex': 35}
        for d in ('middle', 'ring', 'pinky'):
            h[side][d] = dict(DOME, mcp_abd=0)
        h[side]['thumb'] = {'cmc_flex': 15, 'cmc_abd': 25, 'cmc_rot': 20, 'mcp_flex': 15, 'ip_flex': 20}
    return h


V_SNOUT = ['right.index.*', 'left.index.*']
GOALS = [{'touch': ['right.index.tip', 'left.index.tip'], 'vary': V_SNOUT},
         {'touch': ['right.thumb.tip', 'right.index.middle'], 'vary': ['right.thumb.*']},
         {'touch': ['left.thumb.tip', 'left.index.middle'], 'vary': ['left.thumb.*']},
         {'vary': ['%s.%s.*' % (s, d) for s in ('right', 'left') for d in ('middle', 'ring', 'pinky')]}]

if __name__ == '__main__':
    for name, kw in (('h2a', {}), ('h2b', {'lean': 0.5})):
        code, text, sol = solve(name, {'version': 1, 'hands': hands2(**kw), 'camera': FRONT, 'goals': GOALS})
        print(name, code, [l.strip()[:110] for l in text.splitlines() if 'MISS' in l or 'penetr' in l])
        still(name, sol)
        still(name + '_side', sol, cam={'position': [0.34, 0.1, 0.05], 'target': [0.0, 0.09, 0.0], 'fov': 30})
