"""The viper's head poses from the clip, as library gestures (crates/handkit-core/gestures):
hand_viper (jaw shut, 6.5 s) and hand_viper_open (striking, jaw wide, 7.0 s)."""
import json, subprocess
from hk3 import *

GDIR = os.path.join(ROOT, 'crates', 'handkit-core', 'gestures')
HEAD = ['right.index.*', 'left.index.*', 'right.thumb.*', 'left.thumb.*']
DESC = {
    'hand_viper': "Hand viper (snake head), jaw shut: the hands clasped with middle, ring and little fingers interlaced over each other's backs, the index fingers curled forward over the front, one on the other (the snout), and both thumbs forward underneath (the lower jaw), the index tips resting on them; forearms up from below. Seen from the side the head points to the right.",
    'hand_viper_open': "Hand viper (snake head) striking: the hand_viper clasp with the snout lifted and the thumbs dropped, so the mouth opens wide; seen from the side the head points to the right.",
}
CAM = {'position': [0.27, 0.085, 0.30], 'target': [0.06, 0.07, 0.0], 'fov': 30}


def at(t):
    c = json.load(open(os.path.join(HERE, 'clip.json')))
    c['idle'] = 0
    path = os.path.join(OUT, 'clip_noidle.json')
    dump(path, c)
    text = run([H, 'sample', path, str(t), '--wrist'])[1]
    return json.loads(text[text.index('{'):])['hands']


def r(x):
    return round(x, 1) if abs(x) >= 0.05 else 0


for name, t in (('hand_viper', 6.5), ('hand_viper_open', 7.0)):
    hands = {}
    for side, h in at(t).items():
        out = {'place': {k: [round(v, 4) + 0.0 for v in h['place'][k]] for k in ('wrist', 'palm_facing', 'fingers_toward', 'forearm_toward') if k in h['place']}}
        for k in ('twist', 'wrist_flex', 'wrist_dev', 'palm_cup'):
            if k in h:
                out[k] = r(h[k])
        for d in ('thumb', 'index', 'middle', 'ring', 'pinky'):
            out[d] = {k: r(v) for k, v in h[d].items()}
        hands[side.split('.', 1)[1]] = out
    g = {'name': name, 'description': DESC[name], 'version': 1, 'hands': hands,
         'goals': [{'vary': HEAD}], 'camera': CAM}
    path = os.path.join(GDIR, name + '.json')
    # The library's layout: one line per placement and per digit.
    line = lambda v: json.dumps(v, separators=(', ', ': ')).replace('{"', '{ "').replace('}', ' }')
    out = ['{', '  "name": %s,' % json.dumps(name), '  "description": %s,' % json.dumps(DESC[name]), '  "version": 1,', '  "hands": {']
    for i, (side, h) in enumerate(hands.items()):
        out.append('    "%s": {' % side)
        out += ['      "%s": %s%s' % (k, line(v), ',' if j < len(h) - 1 else '') for j, (k, v) in enumerate(h.items())]
        out.append('    }' + (',' if i < len(hands) - 1 else ''))
    out += ['  },', '  "goals": [', '    %s' % line(g['goals'][0]), '  ],', '  "camera": %s' % line(CAM), '}', '']
    with open(path, 'w', newline='\n') as f:
        f.write('\n'.join(out))
    assert json.load(open(path)) == json.loads(json.dumps(g))
    print(name, run([H, 'query', path])[1].split('contacts')[0])
