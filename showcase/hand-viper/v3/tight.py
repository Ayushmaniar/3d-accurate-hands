"""A tighter clasp: the left hand nearer the right one than the grip's preset (z in mm), so the
fingers curl further to reach the other hand's back. Writes upright_hands_z<z>.json + stills."""
import sys, json
from hk3 import *
import layout, clasp, upright
from vgrip import row

z = float(sys.argv[1]) / 1000
layout.OFFSET = (0.010, 0.0, z)
code, text, before, hands = clasp.clasp()
print(code, '\n'.join(text.splitlines()[:2])[:600])
up = upright.stand(hands)
q = query(pose(up))
pens = q.split('penetrations')[1].split('contacts')[0].strip().splitlines()[1:]
print('pen', len(pens), [p.strip() for p in pens[:5]])
tag = 'z%d' % round(z * 1000)
dump(os.path.join(OUT, 'upright_hands_%s.json' % tag), up)
row('tight_' + tag, up)
