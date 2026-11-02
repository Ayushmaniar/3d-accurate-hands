"""Step 1: crossed wrists, palms pressed together, fingers straight forward, thumbs underneath."""
import math
from hk3 import *

# Palm surfaces sit 13 mm in front of the hand frame, so the frames are 26 mm apart when the
# palms touch. The left hand sits 9 mm higher (toward the pinkies) so the fingers alternate.
FLAT = {d: {'mcp_flex': 4, 'pip_flex': 6, 'dip_flex': 3} for d in ('index', 'middle', 'ring', 'pinky')}
THUMB = {'cmc_flex': -5, 'cmc_abd': 5, 'cmc_rot': 5, 'mcp_flex': 8, 'ip_flex': 8}


# The right wrist rides 24 mm above the left one; each hand tilts 10 degrees toward the other.
LIFT, TILT = 0.024, math.radians(10)


def hands(dy=0.0):
    rw, lw = [0.0, LIFT, -0.013], [0.0, -LIFT + dy, 0.013]
    c, s = math.cos(TILT), math.sin(TILT)
    h = {'me.right': dict(copy.deepcopy(FLAT), thumb=dict(THUMB), place=place('right', rw, [0, 0, 1], [c, -s, 0])),
         'me.left': dict(copy.deepcopy(FLAT), thumb=dict(THUMB), place=place('left', lw, [0, 0, -1], [c, s, 0]))}
    return h


if __name__ == '__main__':
    h = hands()
    print(query(pose(h)))
    print(still('base_side', h)[1][-300:])
    print(still('base_front', h, FRONT)[1][-300:])
