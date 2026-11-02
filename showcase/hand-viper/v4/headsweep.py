"""Head placements from step 5's knot: palms turned toward each other by c, fingers leaning in
by a, wrists `half` from the middle; each solved with the arch, the jaw and the knot kept clear."""
import copy
from steps import *
from mouth import ARCH, SHUT

_, _, _, _, _, s5 = build()
TOP = {'position': [0.0, 0.09, 0.42], 'target': [0.0, 0.09, 0.0], 'fov': 26}
names = []
for a, c, half in ((15, -55, 0.056), (15, -40, 0.05), (20, -40, 0.048), (20, -30, 0.045)):
    h = copy.deepcopy(s5)
    pl = oriented(a, 0, c, half)
    for s in S:
        h[s]['place'] = dict(pl[s]['place'], forearm_toward=h[s]['place']['forearm_toward'])
    n = 'hs_%d_%d_%d' % (a, -c, half * 1000)
    code, text, sol = solve(n, {'version': 1, 'hands': h, 'goals': [ARCH] + SHUT + [clear('middle', 'ring', 'pinky')]})
    q = run([H, 'query', os.path.join(OUT, n + '_sol.json')])[1].split('penetrations')[1].split('contacts')[0]
    print(n, len([l for l in text.splitlines() if 'MISS' in l and 'clear' not in l]), [l.strip() for l in q.splitlines() if 'mm' in l][:2])
    still(n, {'version': 1, 'hands': sol['hands']}, cam=TOP, size=360)
    names.append(n)
print(' '.join(names))
