"""The clasp stood up like the reference: forearms up from below, fingers up, both thumbs
toward +x (where the head points). From the side camera (+z) we see the back of the left hand
and the head points screen right; FACE looks the snake in the eye from +x."""
import json, math
from hk3 import *

SIDE_UP = {'position': [0.03, 0.10, 0.40], 'target': [0.01, 0.07, 0.0], 'fov': 30}
FACE = {'position': [0.40, 0.12, 0.06], 'target': [0.01, 0.07, 0.0], 'fov': 30}
# The forearms lean back (-x, under the tail) and splay a little apart.
ARM = {'right': [-0.30, -0.95, -0.10], 'left': [-0.30, -0.95, 0.10]}


def turn(v):
    """The clasp's frame (fingers +x, thumbs -y) to the upright one (fingers +y, thumbs +x)."""
    return [-v[1], v[0], v[2]]


def stand(hands):
    out = copy.deepcopy(hands)
    for side in ('right', 'left'):
        pl = out['me.' + side]['place']
        for k in ('wrist', 'palm_facing', 'fingers_toward'):
            pl[k] = turn(pl[k])
        w = pl['wrist']
        ELBOW[side] = [w[i] + 0.26 * ARM[side][i] for i in range(3)]
        pl['forearm_toward'] = ARM[side]
    return out


if __name__ == '__main__':
    h = stand(json.load(open(os.path.join(OUT, 'clasp_hands.json'))))
    dump(os.path.join(OUT, 'upright_hands.json'), h)
    print('\n'.join(query(pose(h)).splitlines()[:6]))
    still('up_side', h, SIDE_UP)
    still('up_face', h, FACE)
