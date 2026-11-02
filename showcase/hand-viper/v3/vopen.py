"""Open the upright clasp into the reference's V: each hand turns about the heels of the palms
(an upright axis on the pinky side), palms opening toward the head, a few degrees at a time,
the laced fingers re-solved onto the other hand's back after each turn."""
import json, math, sys
from hk3 import *
from upright import SIDE_UP, FACE
from head import THREE_Q, report

SIDES = ('right', 'left')
HINGE = [-0.035, 0.0, 0.0]
# Where the grip landed each fingertip (clasp.py); the index fingers are left free for the snout.
LACE = [('me.right.middle.distal', 'me.left.ring.metacarpal'), ('me.right.ring.distal', 'me.left.pinky.metacarpal'),
        ('me.right.pinky.distal', 'me.left.pinky.metacarpal'), ('me.left.middle.distal', 'me.right.middle.metacarpal'),
        ('me.left.ring.distal', 'me.right.ring.metacarpal'), ('me.left.pinky.distal', 'me.right.pinky.metacarpal')]
LACED = ['me.%s.%s.*' % (s, d) for s in SIDES for d in ('middle', 'ring', 'pinky')] + ['me.right.palm_cup', 'me.left.palm_cup']


def spin(v, a, about=None):
    c, s = math.cos(a), math.sin(a)
    o = about or [0, 0, 0]
    x, z = v[0] - o[0], v[2] - o[2]
    return [o[0] + x * c + z * s, v[1], o[2] - x * s + z * c]


def turn_hands(h, deg):
    out = copy.deepcopy(h)
    for side, sign in (('right', 1), ('left', -1)):
        pl = out['me.' + side]['place']
        a = math.radians(deg) * sign
        pl['wrist'] = spin(pl['wrist'], a, HINGE)
        for k in ('palm_facing', 'fingers_toward', 'forearm_toward'):
            pl[k] = spin(pl[k], a)
    return out


def lace(h, tag):
    goals = [{'touch': [a, b], 'vary': LACED} for a, b in LACE]
    code, text, s = solve('open', pose(h, goals))
    report(tag, code, text, s['hands'])
    return s['hands']


if __name__ == '__main__':
    target, step = float(sys.argv[1]) if len(sys.argv) > 1 else 30, 6
    h = json.load(open(os.path.join(OUT, 'upright_hands.json')))
    done = 0
    while done < target:
        d = min(step, target - done)
        h = lace(turn_hands(h, d), 'open %g' % (done + d))
        done += d
    dump(os.path.join(OUT, 'open_hands.json'), h)
    for n, cam in (('side', SIDE_UP), ('face', FACE), ('3q', THREE_Q)):
        still('open_' + n, h, cam)
    from PIL import Image
    ims = [Image.open(os.path.join(OUT, 'open_%s.png' % n)) for n in ('side', 'face', '3q')]
    row = Image.new('RGB', (sum(i.size[0] for i in ims), ims[0].size[1]))
    for i, im in enumerate(ims):
        row.paste(im, (i * im.size[0], 0))
    row.save(os.path.join(OUT, 'open_row.png'))
