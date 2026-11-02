"""Naruto's tiger seal (tora): palms together, ring and little fingers interlaced over the backs,
index and middle fingers straight up and pressed together, the thumbs straight up side by side."""
from sym import *

THUMB = {"cmc_flex": 15, "cmc_abd": -5, 'cmc_rot': 0, 'mcp_flex': 0, 'ip_flex': 0}
UP = {'mcp_abd': 0, 'mcp_flex': 0, 'pip_flex': 0, 'dip_flex': 0}


def hands(gap=0.05, dz=0.004, lean=0.15):
    h = {}
    for side, s in (('right', -1), ('left', 1)):
        h[side] = {'place': {'wrist': [s * gap / 2, 0.0, s * dz], 'palm_facing': [-s, 0, 0], 'fingers_toward': [-s * lean, 1, 0],
                             'forearm_toward': [s * 0.25, -1, -0.45]},
                   'thumb': dict(THUMB), 'index': dict(UP), 'middle': dict(UP)}
    return h


CAM = {'position': [0.10, 0.16, 0.34], 'target': [0.0, 0.09, 0.0], 'fov': 30}
if __name__ == '__main__':
    pose = {'version': 1, 'hands': hands(), 'camera': CAM,
            'goals': [{'grip': ['right', 'left'], 'style': 'interlaced',
                       'vary': ['right.ring.*', 'right.pinky.*', 'left.ring.*', 'left.pinky.*']},
                      # The thumbs pressed side by side; the left hand settles to make room.
                      {'touch': ['right.thumb.side_palmar', 'left.thumb.side_palmar'],
                       'vary': ['right.thumb.*', 'left.thumb.*', 'left.place.position']}]}
    code, text, sol = solve('tiger', pose)
    print(code, text[-1500:])
    if sol:
        print(query('tiger', sol)[:1800])
        print(still('tiger', sol)[1][-200:])
        print(still('tiger_back', sol, cam=[-0.12, 0.16, -0.3, 0, 0.09, 0, 30])[1][-200:])
