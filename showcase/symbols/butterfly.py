"""The butterfly: wrists crossed with the palms toward the person, the thumbs hooked round each
other as the body, the fingers spread as wings."""
from sym import *

SPREAD = {'index': {'mcp_abd': 18}, 'middle': {'mcp_abd': 6}, 'ring': {'mcp_abd': 12}, 'pinky': {'mcp_abd': 22}}


def hands(open_deg=0.0, wing=0.55):
    """Wings at `open_deg` (0 spread flat, positive folds them toward the person)."""
    h = {}
    for side, s in (('right', 1), ('left', -1)):
        # Palm toward the person (-z), fingers up and out; crossed: the right hand on the right.
        h[side] = {'place': {'wrist': [s * 0.012, 0.0, 0.006 * s], 'palm_facing': [0, 0, -1], 'fingers_toward': [s * wing, 1, 0],
                             'forearm_toward': [-s * 0.55, -1, -0.5]},
                   'wrist_flex': -open_deg,
                   'thumb': {'cmc_flex': -10, 'cmc_abd': 20, 'cmc_rot': 10, 'mcp_flex': 25, 'ip_flex': 50}}
        for d, v in SPREAD.items():
            h[side][d] = dict(v, mcp_flex=-5, pip_flex=5, dip_flex=3)
    return h


CAM = {'position': [0.0, 0.10, 0.38], 'target': [0.0, 0.07, 0.0], 'fov': 34}
GOALS = [{'touch': ['right.thumb.pad', 'left.thumb.proximal'], 'vary': ['right.thumb.*', 'left.thumb.*', 'left.place.position']},
         {'touch': ['left.thumb.pad', 'right.thumb.proximal'], 'vary': ['right.thumb.*', 'left.thumb.*', 'left.place.position']}]
if __name__ == '__main__':
    pose = {'version': 1, 'hands': hands(), 'camera': CAM, 'goals': GOALS}
    code, text, sol = solve('butterfly', pose)
    print(code, '\n'.join(l for l in text.splitlines() if '->' not in l)[-1400:])
    if sol:
        print(query('butterfly', sol).split('contacts')[0][-1200:])
        still('butterfly', sol)
        still('butterfly_low', sol, cam=[0.05, -0.02, 0.3, 0, 0.06, 0, 34])
