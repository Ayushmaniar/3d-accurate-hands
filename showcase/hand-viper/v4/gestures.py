"""The front-facing viper head (tutclip.py's solved head) as library gestures, replacing the
side-on ones: hand_viper (mouth shut) and hand_viper_open (thumbs dropped, mouth wide)."""
import json, os, subprocess
from steps import H, OUT, run

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
GDIR = os.path.join(ROOT, 'crates', 'handkit-core', 'gestures')
CAM = {'position': [0.0, 0.09, 0.42], 'target': [0.0, 0.085, 0.0], 'fov': 28}
ARCH = {'touch': ['right.index.tip', 'left.index.tip'], 'vary': ['right.index.*', 'left.index.*']}
JAW = [{'touch': ['%s.index.tip' % s, '%s.thumb.tip' % s], 'vary': ['%s.index.*' % s, '%s.thumb.*' % s]} for s in ('right', 'left')]
DESC = {
    'hand_viper': "Hand viper (snake head) facing the viewer, mouth shut: both palms cupped toward the viewer, wrists close in a V; little fingers curled into the palms, ring and middle fingers crossed and hooked into a knot inside; the index fingers curled into an arch on top, tips meeting (the upper jaw), each thumb bent up under its own index tip (the lower jaw). Built as youtube ef1n1x2WlZ0 shows.",
    'hand_viper_open': "Hand viper facing the viewer, mouth open: the hand_viper head with both thumbs dropped down and out, away from the index tips, which lift a little.",
}
SRC = {'hand_viper': ('tut_head_sol.json', [ARCH] + JAW), 'hand_viper_open': ('tut_open10_sol.json', [ARCH])}


def r(x):
    return round(x, 1) if abs(x) >= 0.05 else 0


line = lambda v: json.dumps(v, separators=(', ', ': ')).replace('{"', '{ "').replace('}', ' }')
for name, (src, goals) in SRC.items():
    hands = {}
    # Solved against the gesture's own goals, so the library file meets them.
    pre = os.path.join(OUT, 'g_' + name + '.json')
    json.dump({'version': 1, 'hands': json.load(open(os.path.join(OUT, src)))['hands'], 'goals': goals}, open(pre, 'w'))
    print(run([H, 'solve', pre, '-o', pre.replace('.json', '_sol.json')])[1].splitlines()[0])
    for side, h in json.load(open(pre.replace('.json', '_sol.json')))['hands'].items():
        out = {'place': {k: [round(v, 4) + 0.0 for v in h['place'][k]] for k in ('wrist', 'palm_facing', 'fingers_toward', 'forearm_toward') if k in h['place']}}
        for d in ('thumb', 'index', 'middle', 'ring', 'pinky'):
            out[d] = {k: r(v) for k, v in h[d].items()}
        hands[side] = out
    text = ['{', '  "name": %s,' % json.dumps(name), '  "description": %s,' % json.dumps(DESC[name]), '  "version": 1,', '  "hands": {']
    for i, (side, h) in enumerate(hands.items()):
        text.append('    "%s": {' % side)
        text += ['      "%s": %s%s' % (k, line(v), ',' if j < len(h) - 1 else '') for j, (k, v) in enumerate(h.items())]
        text.append('    }' + (',' if i < len(hands) - 1 else ''))
    text += ['  },', '  "goals": [']
    text += ['    %s%s' % (line(g), ',' if j < len(goals) - 1 else '') for j, g in enumerate(goals)]
    text += ['  ],', '  "camera": %s' % line(CAM), '}', '']
    path = os.path.join(GDIR, name + '.json')
    with open(path, 'w', newline='\n') as f:
        f.write('\n'.join(text))
    q = run([H, 'query', path])[1]
    print(name, [l.strip()[:90] for l in q.splitlines() if 'MISS' in l or ' ok ' in l], q.split('penetrations')[1].split('contacts')[0].strip()[:60])
