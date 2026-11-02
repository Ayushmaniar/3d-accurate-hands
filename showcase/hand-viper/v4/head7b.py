from head7 import *
names = []
for lean in (0.9, 1.3):
    code, text, sol = solve('h7b_%d' % (lean * 10), head((25, 90, 40), 'pad', lean=lean))
    print(lean, [l.strip()[:80] for l in text.splitlines() if 'MISS' in l or 'penetr' in l])
    for cy in (0.10, 0.17):
        n = 'h7b_%d_%d' % (lean * 10, cy * 100)
        still(n, sol, cam={'position': [0.0, cy, 0.37], 'target': [0.0, 0.075, 0.0], 'fov': 28}, size=360)
        names.append(n)
print(' '.join(names))
