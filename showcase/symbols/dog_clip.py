"""The shadow-puppet dog barking on a wall: the jaw (little finger) snaps open and shut, the ear
(thumb) twitches, the head lifts for a howl."""
import copy
from sym import *

shut = json.load(open(os.path.join(OUT, 'dog_sol.json')))['hands']['right']


def head(jaw=0.0, ear=0.0, lift=0.0):
    h = copy.deepcopy(shut)
    h['pinky']['mcp_abd'] = round(shut['pinky']['mcp_abd'] + jaw, 1)
    h['pinky']['mcp_flex'] = round(shut['pinky']['mcp_flex'] + 0.3 * jaw, 1)
    h['ring']['mcp_flex'] = round(shut['ring']['mcp_flex'] + 0.15 * jaw, 1)
    h['thumb']['cmc_flex'] = round(min(50, shut['thumb']['cmc_flex'] + ear), 1)
    h['thumb']['ip_flex'] = round(shut['thumb']['ip_flex'] + 0.6 * ear, 1)
    h['place'] = dict(h['place'], fingers_toward=[1, 0.08 + lift, 0])
    return {'me.right': h}


keys = [{'t': 0, 'hands': head()}]
t = 0.6
for jaw in (32, 28):
    keys += [{'t': t, 'hands': head(jaw), 'move': 0.12, 'ease': 'snappy'}, {'t': t + 0.2, 'hands': head(0), 'move': 0.14, 'ease': 'snappy'}]
    t += 0.45
keys += [{'t': t + 0.3, 'hands': head(0, ear=18), 'move': 0.15}, {'t': t + 0.5, 'hands': head(0), 'move': 0.15}]
t += 0.9
keys += [{'t': t + 0.6, 'hands': head(26, lift=0.45), 'move': 0.6, 'ease': 'smooth'},
         {'t': t + 1.8, 'hands': head(30, lift=0.5), 'move': 1.0},
         {'t': t + 2.4, 'hands': head(0), 'move': 0.5}]
clip = {'version': 1, 'fps': 30, 'duration': round(t + 3.0, 2), 'idle': {'amount': 0.12},
        'camera': {'position': [0.12, 0.2, 0.55], 'target': [0.08, 0.1, 0.0], 'fov': 34},
        'look': dict(LOOK, motion_blur=3, wall={'behind': 0.3}), 'keys': keys}
path = os.path.join(HERE, 'dog_clip.json')
dump(path, clip)
print(run([H, 'check', path])[1][-900:])
