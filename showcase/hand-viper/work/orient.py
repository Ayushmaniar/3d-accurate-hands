"""Turn knot-frame poses (built palms +-X, fingers +Y, thumbs -Z, as in steps.py) into the
film's world: the knot rolled onto its side (left hand on top, palm down; right hand below,
palm up), fingers pointing forward = -X, which the side camera at -Z shows as screen right.
Forearms are re-aimed in the world so they come in from the back (+X), level."""
import math, copy

HANDS = ('right', 'left')


def mat_mul(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(3)) for j in range(3)] for i in range(3)]


def apply(m, v):
    return [sum(m[i][k] * v[k] for k in range(3)) for i in range(3)]


def rx(d):
    a = math.radians(d); c, s = math.cos(a), math.sin(a)
    return [[1, 0, 0], [0, c, -s], [0, s, c]]


def ry(d):
    a = math.radians(d); c, s = math.cos(a), math.sin(a)
    return [[c, 0, s], [0, 1, 0], [-s, 0, c]]


def rz(d):
    a = math.radians(d); c, s = math.cos(a), math.sin(a)
    return [[c, -s, 0], [s, c, 0], [0, 0, 1]]


def orient(roll=90.0, pitch=0.0, yaw=0.0):
    """roll: degrees the knot turns about its finger axis (0 = palms vertical, prayer;
    90 = left palm down on top). Then fingers (+Y) are laid forward to -X and pitched up by
    `pitch` (positive raises the fingertips), and yawed about world Y by `yaw`."""
    # roll about the knot's finger axis (knot Y): at 90 nothing turns (the lay-down below
    # already puts the right palm normal, knot +X, to world +Y); at 0 the thumbs point up.
    R_roll = ry(roll - 90)
    # lay down: knot Y -> world -X, knot Z -> world Z, knot X -> world Y (rotation about Z by +90)
    R_lay = rz(90)
    R = mat_mul(rz(-pitch), R_lay)   # positive pitch lifts the fingertips (-X end) up
    R = mat_mul(R, R_roll)
    R = mat_mul(ry(yaw), R)
    return R


def transform(hands, R, center=(0, 0.1, -0.01), at=(0, 0, 0), forearms=None):
    """Rotate both hands' placements by R about the knot point `center`, put that point at
    `at`, and set world forearm directions {'right': [..], 'left': [..]}."""
    h = copy.deepcopy(hands)
    for side in HANDS:
        pl = h[side]['place']
        w = [pl['wrist'][i] - center[i] for i in range(3)]
        w = apply(R, w)
        new = {'wrist': [round(w[i] + at[i], 5) for i in range(3)],
               'palm_facing': [round(x, 5) for x in apply(R, pl['palm_facing'])],
               'fingers_toward': [round(x, 5) for x in apply(R, pl['fingers_toward'])]}
        if forearms:
            new['forearm_toward'] = forearms[side]
        else:
            new['forearm_toward'] = [round(x, 5) for x in apply(R, pl['forearm_toward'])]
        h[side]['place'] = new
        for k in ('wrist_flex', 'wrist_dev'):
            h[side].pop(k, None)
    return h


def orient_profile(tilt=12.0):
    """Reference layout: the knot stays upright (fingers up), seen side-on from the back of
    the left hand, with the head (the thumb side, knot -Z) pointing screen right and raised by
    `tilt` degrees; the fingers lean back to the left by the same angle. The camera sits at
    -Z looking +Z, so screen right is world -X."""
    t = math.radians(tilt); c, s = math.cos(t), math.sin(t)
    X = [0, 0, -1]; Y = [s, c, 0]; Z = [c, -s, 0]      # images of knot X, Y, Z
    return [[X[i], Y[i], Z[i]] for i in range(3)]
