"""The shadow-puppet dog (the one-hand wolf): the back of the hand toward the light, fingers
forward as the snout, the thumb up as the ear, the little finger dropping as the jaw."""
from sym import *

CAM = {'position': [0.03, 0.15, 0.5], 'target': [0.05, 0.14, 0.0], 'fov': 30}
SNOUT_VARY = ['right.index.*', 'right.middle.*', 'right.ring.*']
SNOUT = [{'touch': ['right.index.side_ulnar', 'right.middle.side_radial'], 'vary': SNOUT_VARY},
         {'touch': ['right.middle.side_ulnar', 'right.ring.side_radial'], 'vary': SNOUT_VARY}]


def hand(jaw=0.0):
    return {'right': {'place': {'wrist': [-0.04, 0.12, 0.0], 'palm_facing': [0, -0.15, -1], 'fingers_toward': [1, 0.08, 0],
                                'forearm_toward': [-1, -0.35, -0.3]},
                      'thumb': {'cmc_flex': -30, 'cmc_abd': -10, 'cmc_rot': 0, 'mcp_flex': 0, 'ip_flex': -10},
                      'index': {'mcp_abd': -4, 'mcp_flex': 4, 'pip_flex': 22, 'dip_flex': 8},
                      'middle': {'mcp_abd': 2, 'mcp_flex': 0, 'pip_flex': 6, 'dip_flex': 4},
                      'ring': {'mcp_abd': 0, 'mcp_flex': 2 + 0.2 * jaw, 'pip_flex': 6, 'dip_flex': 4},
                      'pinky': {'mcp_abd': 4 + jaw, 'mcp_flex': 4 + 0.3 * jaw, 'pip_flex': 6, 'dip_flex': 4}}}


if __name__ == '__main__':
    for name, jaw in (('dog', 0), ('dog_open', 30)):
        code, text, pose = solve(name, {'version': 1, 'hands': hand(jaw), 'camera': CAM, 'goals': SNOUT})
        print(name, [l for l in text.splitlines() if 'touch' in l or 'penetr' in l])
        still(name, pose)
