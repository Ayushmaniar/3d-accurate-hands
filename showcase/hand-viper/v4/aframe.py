"""The tutorial's A-frame (39.5 s): the wrists apart, each hand's fingers angled in by `apex`
degrees to meet and interlace at the top middle, the palms facing down and in, the thumb sides
to the viewer. Then the head as before: the index arch, the thumbs underneath, the eyes on top."""
import math, sys
from head9 import *


def aframe(apex=40, half=0.06, dz=0.008, lean=0.6):
    h = cupped(half, lean)
    a = math.radians(apex)
    for side, s in (('right', 1), ('left', -1)):
        p = h[side]['place']
        p['wrist'] = [s * half, 0.0, s * dz]
        p['fingers_toward'] = [round(-s * math.sin(a), 4), round(math.cos(a), 4), lean]
        p['palm_facing'] = [round(-s * math.cos(a), 4), round(-math.sin(a), 4), 0]
        p['forearm_toward'] = [s * 0.45, -1, -0.45]
    return h


def build(apex, half, tag, lean=0.6):
    code, text, shut = solve(tag, {'version': 1, 'hands': aframe(apex, half, lean=lean), 'goals': [ARCH, SIDE] + SHUT + [CLEAR]})
    print(tag, [l.strip()[:90] for l in text.splitlines() if 'MISS' in l or 'penetr' in l])
    still(tag + '_f', shut, cam={'position': [0.0, 0.13, 0.42], 'target': [0.0, 0.07, 0.0], 'fov': 28}, size=360)
    still(tag + '_t', shut, cam={'position': [0.0, 0.34, 0.2], 'target': [0.0, 0.07, 0.03], 'fov': 30}, size=360)
    return shut


if __name__ == '__main__':
    for apex, half in ((35, 0.055), (45, 0.065)):
        build(apex, half, 'a_%d' % apex)
