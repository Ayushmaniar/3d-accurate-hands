"""Head grid: dome fold x thumb setup, from h6 (fists 46 mm from the middle, leaning 0.5 toward
the viewer). Thumb setups: 'under' (each thumb tip up against its own index's last segment, the
two tips side by side) or 'pad' (each thumb pad on its own index's middle segment)."""
import sys
from head2 import *

CAM7 = {'position': [0.0, 0.17, 0.37], 'target': [0.0, 0.085, 0.0], 'fov': 28}
THUMB0 = {'cmc_flex': 30, 'cmc_abd': 10, 'cmc_rot': 25, 'mcp_flex': 30, 'ip_flex': 25}
CLEAR = {'vary': ['%s.%s.*' % (s, d) for s in ('right', 'left') for d in ('middle', 'ring', 'pinky')]}


def thumb_goals(kind):
    if kind == 'under':
        return [{'touch': ['%s.thumb.tip' % s, '%s.index.distal' % s], 'vary': ['%s.thumb.*' % s]} for s in ('right', 'left')] + \
               [{'touch': ['right.thumb.tip', 'left.thumb.tip'], 'vary': ['right.thumb.*', 'left.thumb.*']}]
    return [{'touch': ['%s.thumb.pad' % s, '%s.index.middle' % s], 'vary': ['%s.thumb.*' % s]} for s in ('right', 'left')]


def head(dome, kind, half=0.046, lean=0.5):
    DOME.update(zip(('mcp_flex', 'pip_flex', 'dip_flex'), dome))
    h = hands2(half, 0.006, lean)
    for s in h:
        h[s]['thumb'] = dict(THUMB0)
    goals = [{'touch': ['right.index.tip', 'left.index.tip'], 'vary': V_SNOUT}] + thumb_goals(kind) + [CLEAR]
    return {'version': 1, 'hands': h, 'goals': goals}


if __name__ == '__main__':
    for dome in ((25, 90, 40), (40, 95, 50)):
        for kind in ('under', 'pad'):
            name = 'h7_%d_%s' % (dome[0], kind)
            code, text, sol = solve(name, head(dome, kind))
            print(name, [l.strip()[:90] for l in text.splitlines() if 'MISS' in l or 'penetr' in l])
            still(name, sol, cam=CAM7, size=360)
