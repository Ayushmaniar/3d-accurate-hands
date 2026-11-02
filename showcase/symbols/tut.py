"""A finger-tutting box: palms out, each index straight up then bent square at the middle joint
so the two fingertips meet above; the thumbs straight across underneath, tip to tip."""
from sym import *

FIST = {'mcp_flex': 85, 'pip_flex': 95, 'dip_flex': 50}
CAM = {'position': [0.0, 0.12, 0.42], 'target': [0.0, 0.11, 0.0], 'fov': 30}


def hands():
    h = {}
    for side, s in (('right', -1), ('left', 1)):
        h[side] = {'place': {'wrist': [s * 0.06, 0.03, 0.0], 'palm_facing': [0, 0, 1], 'fingers_toward': [0, 1, 0], 'forearm_toward': [s * 0.3, -1, -0.4]},
                   'thumb': {'cmc_flex': -30, 'cmc_abd': 0, 'cmc_rot': 0, 'mcp_flex': 0, 'ip_flex': 0},
                   'index': {'mcp_abd': 0, 'mcp_flex': 0, 'pip_flex': 90, 'dip_flex': 0},
                   'middle': dict(FIST), 'ring': dict(FIST), 'pinky': dict(FIST), 'palm_cup': 10}
    return h


GOALS = [{'touch': ['right.index.tip', 'left.index.tip'], 'vary': ['right.index.*', 'left.index.*', 'left.place.position', 'right.place.position']},
         {'touch': ['right.thumb.tip', 'left.thumb.tip'], 'vary': ['right.thumb.*', 'left.thumb.*', 'right.wrist', 'left.wrist']}]
if __name__ == '__main__':
    code, text, pose = solve('tut', {'version': 1, 'hands': hands(), 'camera': CAM, 'goals': GOALS})
    print([l for l in text.splitlines() if 'touch' in l or 'penetr' in l])
    print(query('tut', pose).split('contacts')[0][-700:])
    still('tut', pose)
    still('tut_side', pose, cam=[0.3, 0.14, 0.3, 0, 0.1, 0, 30])
