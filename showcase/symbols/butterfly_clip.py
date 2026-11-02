"""The butterfly flapping: each hand turns about the body (the hooked thumbs, vertical), wings
folding toward the person and back, then a glide."""
import copy, math
from sym import *

sol = json.load(open(os.path.join(OUT, 'butterfly_sol.json')))


def turn(v, deg):
    a = math.radians(deg)
    return [round(v[0] * math.cos(a) + v[2] * math.sin(a), 5), v[1], round(-v[0] * math.sin(a) + v[2] * math.cos(a), 5)]


def wings(deg, lift=0.0):
    """The wings folded `deg` toward the person: every finger bends at its knuckle (the base a
    little more than the tip, as a wing flexes), the whole hand lifting a touch."""
    out = {}
    for side, s in (('right', 1), ('left', -1)):
        h = copy.deepcopy(sol['hands'][side])
        h['place']['wrist'][1] += lift
        for i, d in enumerate(('index', 'middle', 'ring', 'pinky')):
            h[d]['mcp_flex'] = round(-5 + deg * (0.85 + 0.1 * i), 1)
            h[d]['pip_flex'] = round(5 + deg * 0.35, 1)
            h[d]['dip_flex'] = round(3 + deg * 0.2, 1)
        out['me.' + side] = h
    return out


keys = [{'t': 0, 'hands': wings(10)}]
t = 0.0
for i in range(6):
    t += 0.28
    keys.append({'t': round(t, 2), 'hands': wings(55, 0.006), 'move': 0.26, 'ease': 'smooth'})
    t += 0.28
    keys.append({'t': round(t, 2), 'hands': wings(5, -0.004), 'move': 0.26, 'ease': 'smooth'})
keys.append({'t': round(t + 0.8, 2), 'hands': wings(25), 'move': 0.7})
clip = {'version': 1, 'fps': 30, 'duration': round(t + 1.2, 2), 'size': [1080, 1080], 'idle': {'amount': 0.1},
        'camera': {'position': [0.08, 0.17, 0.56], 'target': [0.0, 0.08, 0.0], 'fov': 32},
        'look': dict(LOOK, motion_blur=3), 'keys': keys}
path = os.path.join(HERE, 'butterfly_clip.json')
dump(path, clip)
print(run([H, 'check', path])[1][-1500:])
