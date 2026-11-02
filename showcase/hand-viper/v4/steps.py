"""The hand viper step by step, following ONE tutorial: youtube ef1n1x2WlZ0 (the camera looks
straight down at both palms, so every finger shows). The camera here faces the palms the same
way: palms toward +z, fingers +y, the right hand on screen right, little-finger edges together.
  s1 25.7 s  palms out, little-finger edges together, fingers straight
  s2 27 s    both little fingers curl down into the palms, tips touching
  s3 28.5 s  the ring fingers cross, the left one over (nearer the camera) the right
  s4 34.5 s  the middle fingers cross over them and hook: a knot in the middle
  s5 36.5 s  the index fingers curl in and meet at the top: the arch
Each step keeps the previous one's fingers and is solved with touches; the solver keeps every
finger out of the others."""
import copy, sys
from sweep import *

FRONT = {'position': [0.0, 0.09, 0.42], 'target': [0.0, 0.085, 0.0], 'fov': 30}
FINGERS = ('index', 'middle', 'ring', 'pinky')
S = ('right', 'left')


def flat(half=0.041):
    h = oriented(0, 0, -90, half)
    for s in S:
        for d in FINGERS:
            h[s][d] = {'mcp_abd': 0, 'mcp_flex': 3, 'pip_flex': 4, 'dip_flex': 2}
        h[s]['thumb'] = {'cmc_flex': -15, 'cmc_abd': 0, 'cmc_rot': 0, 'mcp_flex': 0, 'ip_flex': 0}
        h[s]['place']['forearm_toward'] = [(1 if s == 'right' else -1) * 0.15, -1, -0.5]
    return h


def clear(*digits):
    return {'vary': ['%s.%s.*' % (s, d) for s in S for d in digits]}


def step(name, hands, goals):
    code, text, sol = solve(name, {'version': 1, 'hands': hands, 'goals': goals})
    miss = [l.strip()[:80] for l in text.splitlines() if 'MISS' in l]
    pen = run([H, 'query', os.path.join(OUT, name + '_sol.json')])[1].split('penetrations')[1].split('contacts')[0]
    print(name, miss, pen.strip().splitlines()[:3])
    still(name, {'version': 1, 'hands': sol['hands']}, cam=FRONT, size=380)
    return sol['hands']


def build():
    s1 = flat()
    h = copy.deepcopy(s1)
    for s in S:
        h[s]['pinky'] = {'mcp_abd': 0, 'mcp_flex': 60, 'pip_flex': 100, 'dip_flex': 45}
    s2 = step('s2', h, [{'touch': ['right.pinky.tip', 'left.pinky.tip'], 'vary': ['right.pinky.*', 'left.pinky.*']}])
    h = copy.deepcopy(s2)
    # Ring abduction spreads toward the little finger: here, toward the other hand. The hands
    # angle their fingers in a little, as the tutorial's do once the crossing starts.
    lean = oriented(8, 0, -90, 0.055)
    for s in S:
        h[s]['place'] = dict(lean[s]['place'], forearm_toward=h[s]['place']['forearm_toward'])
    h['left']['ring'] = {'mcp_abd': 18, 'mcp_roll': 0, 'mcp_flex': 40, 'pip_flex': 30, 'dip_flex': 15}
    h['right']['ring'] = {'mcp_abd': 18, 'mcp_roll': 0, 'mcp_flex': 20, 'pip_flex': 15, 'dip_flex': 8}
    s3 = step('s3', h, [{'touch': ['left.ring.middle', 'right.ring.middle'], 'vary': ['right.ring.*', 'left.ring.*']}, clear('ring', 'pinky')])
    h = copy.deepcopy(s3)
    # The middle fingers cross like the ring fingers, the left over the right (32.5 s)...
    h['left']['middle'] = {'mcp_abd': -15, 'mcp_roll': 0, 'mcp_flex': 45, 'pip_flex': 30, 'dip_flex': 15}
    h['right']['middle'] = {'mcp_abd': -15, 'mcp_roll': 0, 'mcp_flex': 25, 'pip_flex': 15, 'dip_flex': 8}
    s4a = step('s4a', h, [{'touch': ['left.middle.middle', 'right.middle.middle'], 'vary': ['right.middle.*', 'left.middle.*']}, clear('middle', 'ring', 'pinky')])
    # ...then the crossed middle and ring fingers hook round each other: a knot (34.5 s).
    h = copy.deepcopy(s4a)
    for s in S:
        for d in ('middle', 'ring'):
            h[s][d] = dict(h[s][d], pip_flex=h[s][d]['pip_flex'] + 50, dip_flex=h[s][d]['dip_flex'] + 30)
    s4 = step('s4', h, [clear('middle', 'ring', 'pinky')])
    # The hands turn in and cup (the palms half toward each other) as the index fingers curl in
    # and meet at the top (36.5 s).
    h = copy.deepcopy(s4)
    cup = oriented(15, 0, -55, 0.056)
    for s in S:
        h[s]['place'] = dict(cup[s]['place'])
        h[s]['index'] = {'mcp_abd': -10, 'mcp_roll': 0, 'mcp_flex': 40, 'pip_flex': 60, 'dip_flex': 30}
    s5 = step('s5', h, [{'touch': ['right.index.tip', 'left.index.tip'], 'vary': ['right.index.*', 'left.index.*']}, clear('middle', 'ring', 'pinky')])
    still('s1', {'version': 1, 'hands': s1}, cam=FRONT, size=380)
    return s1, s2, s3, s4a, s4, s5


if __name__ == '__main__':
    build()
