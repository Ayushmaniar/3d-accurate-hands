"""Stills of the clip at one time from several cameras, side by side (out/cams_<t>.png)."""
import json, sys
from hk3 import *

t = sys.argv[1]
code, text = run([H, 'sample', os.path.join(HERE, 'clip.json'), t])
hands = json.loads(text[text.index('{'):])['hands']
T = [0.05, 0.075, 0.0]
CAMS = {'a': ([0.06, 0.03, 0.36], 28), 'b': ([0.14, 0.02, 0.33], 28), 'c': ([0.22, 0.0, 0.28], 28), 'd': ([0.06, 0.03, -0.36], 28)}
from PIL import Image
ims = []
for n, (d, fov) in CAMS.items():
    cam = {'position': [T[i] + d[i] for i in range(3)], 'target': T, 'fov': fov}
    still('cam_' + n, hands, cam, 480)
    ims.append(Image.open(os.path.join(OUT, 'cam_%s.png' % n)))
r = Image.new('RGB', (sum(i.size[0] for i in ims), ims[0].size[1]))
for i, im in enumerate(ims):
    r.paste(im, (i * im.size[0], 0))
r.save(os.path.join(OUT, 'cams_%s.png' % t))
