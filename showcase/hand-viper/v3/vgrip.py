"""The V knot by Handkit's own interlaced grip: the upright clasp's hands turned open about the
heels of the palms (vopen.turn_hands), then the grip closes the fingers with both hands kept
where they are (its vary names no placement)."""
import json, sys
from hk3 import *
from upright import SIDE_UP, FACE
from head import THREE_Q, report
import vopen

DIGITS = ['me.%s.%s.*' % (s, d) for s in ('right', 'left') for d in ('thumb', 'index', 'middle', 'ring', 'pinky')]


def vgrip(deg, hinge_x=-0.035):
    vopen.HINGE[0] = hinge_x
    h = vopen.turn_hands(json.load(open(os.path.join(OUT, 'upright_hands.json'))), deg)
    code, text, s = solve('vgrip%g' % deg, pose(h, [{'grip': ['me.right', 'me.left'], 'style': 'interlaced', 'vary': DIGITS}]))
    report('vgrip %g' % deg, code, text, s['hands'])
    print(text.splitlines()[1][:400])
    return s['hands']


def row(name, h):
    from PIL import Image
    for n, cam in (('side', SIDE_UP), ('face', FACE), ('3q', THREE_Q)):
        still('%s_%s' % (name, n), h, cam)
    ims = [Image.open(os.path.join(OUT, '%s_%s.png' % (name, n))) for n in ('side', 'face', '3q')]
    r = Image.new('RGB', (sum(i.size[0] for i in ims), ims[0].size[1]))
    for i, im in enumerate(ims):
        r.paste(im, (i * im.size[0], 0))
    r.save(os.path.join(OUT, name + '_row.png'))


if __name__ == '__main__':
    deg = float(sys.argv[1])
    h = vgrip(deg)
    dump(os.path.join(OUT, 'vgrip_%g.json' % deg), h)
    row('vgrip%g' % deg, h)
