"""Moderate V on the tight clasp, then the head: base -> turn open `deg` about the pinky edge ->
interlaced grip closing the digits in place -> snout + jaw -> shut. Stills from the head side."""
import json, sys
from hk3 import *
from head import report
import vopen
from vgrip import DIGITS
from snake import IV, TV, S
from manual import lean

deg = float(sys.argv[1])
cmc = float(sys.argv[2]) if len(sys.argv) > 2 else -30
tag = 'vh%g' % deg + ('' if len(sys.argv) < 3 else 't%g' % cmc)
h = vopen.turn_hands(json.load(open(os.path.join(OUT, 'upright_hands_z32.json'))), deg)
code, text, s = solve(tag + '_grip', pose(h, [{'grip': ['me.right', 'me.left'], 'style': 'interlaced', 'vary': DIGITS}]))
report(tag + ' grip', code, text, s['hands'])
h = s['hands']
for side in S:
    h['me.' + side]['index'] = {'mcp_abd': 25, 'mcp_roll': 15, 'mcp_flex': 45, 'pip_flex': 90, 'dip_flex': 40}
    h['me.' + side]['thumb'] = {'cmc_flex': cmc, 'cmc_abd': 15, 'cmc_rot': 10, 'mcp_flex': 0, 'ip_flex': 0}
other = {'right': 'left', 'left': 'right'}
goals = [{'touch': ['me.right.index.pad', 'me.left.index.nail'], 'vary': IV},
         {'touch': ['me.right.thumb.side_dorsal', 'me.left.thumb.side_dorsal'], 'vary': TV}]
goals += [{'touch': ['me.%s.index.tip' % x, 'me.%s.thumb.distal' % other[x]], 'vary': IV + TV} for x in S]
code, text, s = solve(tag + '_shut', pose(h, goals))
report(tag + ' shut', code, text, s['hands'])
h = lean(s['hands'], 25)
dump(os.path.join(OUT, tag + '_hands.json'), h)
T = [0.05, 0.075, 0.0]
from PIL import Image
ims = []
for n, d in (('a', [0.30, 0.06, 0.16]), ('h', [0.315, 0.055, 0.14]), ('f', [0.32, 0.03, -0.05]), ('w', [-0.05, 0.08, 0.40])):
    still('%s_%s' % (tag, n), h, {'position': [T[i] + d[i] for i in range(3)], 'target': T, 'fov': 26}, 480)
    ims.append(Image.open(os.path.join(OUT, '%s_%s.png' % (tag, n))))
r = Image.new('RGB', (sum(i.size[0] for i in ims), ims[0].size[1]))
for i, im in enumerate(ims):
    r.paste(im, (i * im.size[0], 0))
r.save(os.path.join(OUT, tag + '_row.png'))
