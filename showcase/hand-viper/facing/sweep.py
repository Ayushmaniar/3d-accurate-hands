"""Tries eye lifts: which puts the two eye knuckles highest, with the least overlap."""
from fk import *
import head

for lift, sink in [(8, 0), (20, 10), (28, 15), (35, 20)]:
    pose = {'version': 1, 'hands': head.hooked(head.G['hands'], lift, sink), 'camera': CAM, 'goals': head.SHUT + [{'vary': head.SPREAD}]}
    code, text, sol = solve('sw_%d_%d' % (lift, sink), pose)
    t = run([H, 'fk', os.path.join(OUT, 'sw_%d_%d_sol.json' % (lift, sink))])[1]
    pips = sorted(((float(l.split('[')[1].split()[1]), l.split(':')[0].strip().replace('_pip', '')) for l in t.splitlines() if '_pip:' in l and 'index' not in l), reverse=True)
    q = query('swq', sol)
    pen = [l for l in q.split('penetrations')[1].split('contacts')[0].splitlines() if l.strip().endswith('mm') and '~' in l]
    deepest = min((float(l.split(':')[-1].split()[0]) for l in pen), default=0)
    miss = [l.strip()[:60] for l in text.splitlines() if 'MISS' in l and abs(float(l.split(':')[1].split()[0])) > 3]
    print(lift, sink, 'top:', [(n, round(y * 1000)) for y, n in pips[:3]], 'pens', len(pen), deepest, 'bad touches', miss)
