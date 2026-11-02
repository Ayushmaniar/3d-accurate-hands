"""The knot the reference makes: hands upright, heels of the palms together, palms opening in a
V toward the head (+x, the thumb side). Middle, ring and pinky interlace over the other hand's
back; the index fingers curl forward into the front of the V (the snout); the thumbs point
forward underneath (the jaw)."""
import json, math, sys
from hk3 import *
from upright import SIDE_UP, FACE
from head import THREE_Q, report

SIDES = ('right', 'left')
WRAP = {'mcp_flex': 55, 'pip_flex': 65, 'dip_flex': 35}
# Fingertip -> the back of the other hand (the left hand sits one finger toward the pinkies).
LACE = [('me.right.middle.pad', 'me.left.ring.metacarpal'), ('me.right.ring.pad', 'me.left.pinky.metacarpal'),
        ('me.left.middle.pad', 'me.right.middle.metacarpal'), ('me.left.ring.pad', 'me.right.ring.metacarpal'),
        ('me.left.pinky.pad', 'me.right.pinky.metacarpal')]


def knot(theta, sep, dy, dx=0.0):
    c, s = math.cos(math.radians(theta)), math.sin(math.radians(theta))
    h = {}
    for side, sz in (('right', -1), ('left', 1)):
        w = [dx if side == 'left' else 0.0, dy if side == 'left' else 0.0, sz * sep / 2]
        hand = {d: dict(WRAP) for d in ('middle', 'ring', 'pinky')}
        hand['index'] = {'mcp_abd': 0, 'mcp_flex': 35, 'pip_flex': 70, 'dip_flex': 40}
        hand['thumb'] = {'cmc_flex': -15, 'cmc_abd': 15, 'cmc_rot': 0, 'mcp_flex': 5, 'ip_flex': 10}
        hand['place'] = {'wrist': w, 'palm_facing': [s, 0, -sz * c], 'fingers_toward': [0, 1, 0],
                         'forearm_toward': [-0.3, -0.95, sz * 0.10]}
        h['me.' + side] = hand
    return h


def lace(h):
    goals = [{'touch': [a, b], 'vary': [a.rsplit('.', 1)[0] + '.*']} for a, b in LACE]
    goals[0]['vary'] = goals[0]['vary'] + ['me.left.place.position']
    code, text, s = solve('lace', pose(h, goals))
    report('lace', code, text, s['hands'])
    print('\n'.join(l for l in text.splitlines() if 'MISS' in l))
    return s['hands']


if __name__ == '__main__':
    theta, sep, dy = [float(a) for a in sys.argv[1:4]] if len(sys.argv) > 3 else (35, 0.05, -0.01)
    h = lace(knot(theta, sep, dy))
    print('left wrist', h['me.left']['place']['wrist'])
    dump(os.path.join(OUT, 'lace_hands.json'), h)
    for n, cam in (('side', SIDE_UP), ('face', FACE), ('3q', THREE_Q)):
        still('lace_' + n, h, cam)
    from PIL import Image
    ims = [Image.open(os.path.join(OUT, 'lace_%s.png' % n)) for n in ('side', 'face', '3q')]
    row = Image.new('RGB', (sum(i.size[0] for i in ims), ims[0].size[1]))
    for i, im in enumerate(ims):
        row.paste(im, (i * im.size[0], 0))
    row.save(os.path.join(OUT, 'lace_row.png'))
