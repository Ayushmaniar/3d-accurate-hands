"""Head as the tutorial's: wrists close in a V, the hands leaning out above them, the index fingers
curled hard into a low round arch, the thumbs bent short at the bottom of the mouth."""
import copy, sys
from steps import *
from mouth import ARCH, SHUT

_, _, _, _, _, s5 = build()
TOP = {'position': [0.0, 0.09, 0.42], 'target': [0.0, 0.09, 0.0], 'fov': 26}
names = []
for a, c, half, tuck in ((-5, -45, 0.036, 30), (-5, -45, 0.036, 50), (-8, -50, 0.034, 45)):
    h = copy.deepcopy(s5)
    pl = oriented(a, 0, c, half)
    for s in S:
        h[s]['place'] = dict(pl[s]['place'], forearm_toward=h[s]['place']['forearm_toward'])
        h[s]['index'] = {'mcp_abd': -5, 'mcp_roll': 0, 'mcp_flex': 70, 'pip_flex': 85, 'dip_flex': 40}
        h[s]['thumb'] = {'cmc_flex': 30, 'cmc_abd': 35, 'cmc_rot': 35, 'mcp_flex': 30, 'ip_flex': 40}
        # The knot tucks under the arch, inside the mouth: middle and ring fold at the knuckle.
        for d in ('middle', 'ring'):
            h[s][d] = dict(h[s][d], mcp_flex=min(90, h[s][d]['mcp_flex'] + tuck))
        h[s]['index'] = {'mcp_abd': -5, 'mcp_roll': 0, 'mcp_flex': 35, 'pip_flex': 90, 'dip_flex': 45}
    n = 'hk_%d_%d_%d' % (a, -c, tuck)
    code, text, sol = solve(n, {'version': 1, 'hands': h, 'goals': [ARCH] + SHUT + [clear('middle', 'ring', 'pinky')]})
    q = run([H, 'query', os.path.join(OUT, n + '_sol.json')])[1].split('penetrations')[1].split('contacts')[0]
    print(n, [l.strip()[:70] for l in text.splitlines() if 'MISS' in l and 'clear' not in l], [l.strip() for l in q.splitlines() if 'mm' in l][:2])
    still(n, {'version': 1, 'hands': sol['hands']}, cam=TOP, size=360)
