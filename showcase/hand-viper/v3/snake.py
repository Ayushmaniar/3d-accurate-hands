"""The viper's head, solved: the upright clasp leaned forward (manual.lean), index fingers
curled over the front and stacked, thumbs forward underneath side by side; the solver only
clears overlaps and closes the touches named."""
import json, sys
from hk3 import *
from head import report
from manual import lean

S = ('right', 'left')
SIDE_V = {'position': [0.06, 0.08, 0.40], 'target': [0.05, 0.05, 0.0], 'fov': 30}
FACE_V = {'position': [0.42, 0.10, 0.08], 'target': [0.05, 0.05, 0.0], 'fov': 30}
TQ_V = {'position': [0.33, 0.13, 0.28], 'target': [0.05, 0.05, 0.0], 'fov': 30}
IDX = {'right': {'mcp_abd': 25, 'mcp_flex': 20, 'pip_flex': 100, 'dip_flex': 40},
       'left': {'mcp_abd': 25, 'mcp_flex': 30, 'pip_flex': 100, 'dip_flex': 40}}
THB = {'cmc_flex': -30, 'cmc_abd': 5, 'cmc_rot': 0, 'mcp_flex': 0, 'ip_flex': 0}
IV, TV = ['me.right.index.*', 'me.left.index.*'], ['me.right.thumb.*', 'me.left.thumb.*']


def rows(name, h, cams=(('side', SIDE_V), ('face', FACE_V), ('3q', TQ_V))):
    from PIL import Image
    for n, cam in cams:
        still('%s_%s' % (name, n), h, cam)
    ims = [Image.open(os.path.join(OUT, '%s_%s.png' % (name, n))) for n, _ in cams]
    r = Image.new('RGB', (sum(i.size[0] for i in ims), ims[0].size[1]))
    for i, im in enumerate(ims):
        r.paste(im, (i * im.size[0], 0))
    r.save(os.path.join(OUT, name + '_row.png'))


def shaped(tilt=30):
    h = json.load(open(os.path.join(OUT, 'upright_hands.json')))
    for s in S:
        h['me.' + s]['index'] = dict(IDX[s])
        h['me.' + s]['thumb'] = dict(THB)
    return lean(h, tilt)


def solve_head(h, goals, tag):
    code, text, s = solve(tag, pose(h, goals, SIDE_V))
    report(tag, code, text, s['hands'])
    print('\n'.join(l for l in text.splitlines() if 'MISS' in l))
    return s['hands']


if __name__ == '__main__':
    h = shaped()
    goals = [{'touch': ['me.right.index.pad', 'me.left.index.nail'], 'vary': IV},
             {'touch': ['me.right.thumb.side_dorsal', 'me.left.thumb.side_dorsal'], 'vary': TV}]
    h = solve_head(h, goals, 'snake')
    for s in S:
        print(s, {k: round(v) for k, v in h['me.' + s]['index'].items()}, {k: round(v) for k, v in h['me.' + s]['thumb'].items()})
    dump(os.path.join(OUT, 'snake_hands.json'), h)
    rows('snake', h)
