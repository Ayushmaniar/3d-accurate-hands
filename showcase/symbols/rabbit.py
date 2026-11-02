"""The shadow-puppet rabbit: one hand, side on to the lamp; index and middle fingers up as the
ears, ring and little fingers curled onto the thumb pad as the face. The ears twitch in the clip."""
import copy
from sym import *

CAM = {'position': [0.05, 0.14, 0.55], 'target': [0.08, 0.13, 0.0], 'fov': 34}
hand = {'place': {'wrist': [-0.02, 0.08, 0.0], 'palm_facing': [0, -0.3, -1], 'fingers_toward': [1, 0.55, 0], 'forearm_toward': [-1, -0.5, -0.3]},
        'index': {'mcp_abd': 6, 'mcp_flex': -15, 'pip_flex': 5, 'dip_flex': 3},
        'middle': {'mcp_abd': -6, 'mcp_flex': -15, 'pip_flex': 5, 'dip_flex': 3},
        'ring': {'mcp_flex': 60, 'pip_flex': 70, 'dip_flex': 35}, 'pinky': {'mcp_flex': 60, 'pip_flex': 75, 'dip_flex': 35},
        'thumb': {'cmc_flex': 30, 'cmc_abd': 40, 'cmc_rot': 40, 'mcp_flex': 20, 'ip_flex': 20}}
GOALS = [{'touch': ['right.ring.tip', 'right.thumb.pad'], 'vary': ['right.ring.*', 'right.thumb.*', 'right.palm_cup']},
         {'touch': ['right.pinky.tip', 'right.thumb.side_palmar'], 'vary': ['right.pinky.*']}]
code, text, sol = solve('rabbit', {'version': 1, 'hands': {'right': hand}, 'camera': CAM, 'goals': GOALS})
print([l.strip()[:90] for l in text.splitlines() if 'MISS' in l or ' ok ' in l or 'penetr' in l])
r = sol['hands']['right']


def ears(fold):
    h = copy.deepcopy(r)
    h['index']['mcp_flex'] = round(r['index']['mcp_flex'] + fold, 1)
    h['middle']['mcp_flex'] = round(r['middle']['mcp_flex'] + fold * 0.6, 1)
    return {'me.right': h}


keys = [{'t': 0, 'hands': ears(0)}]
for i, t in enumerate((0.8, 1.0, 1.9, 2.1, 2.3)):
    keys.append({'t': t, 'hands': ears(40 if i % 2 == 0 else 0), 'move': 0.18, 'ease': 'snappy'})
keys.append({'t': 3.0, 'hands': ears(0), 'move': 0.4})
clip = {'version': 1, 'fps': 30, 'duration': 3.4, 'idle': {'amount': 0.15}, 'camera': CAM,
        'look': dict(LOOK, motion_blur=3, wall={'behind': 0.3}), 'keys': keys}
path = os.path.join(HERE, 'rabbit_clip.json')
dump(path, clip)
print(run([H, 'check', path])[1][-500:])
