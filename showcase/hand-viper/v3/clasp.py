"""The full clasp: the left hand where the interlaced grip puts a follower (layout.py), elbows
from the layout search, then the grip closes the fingers only."""
from hk3 import *
import layout

ELBOW['right'], ELBOW['left'] = [-0.22, 0.14, 0.08], [-0.22, -0.18, -0.08]

FINGERS = ['me.%s.%s.*' % (s, d) for s in ('right', 'left') for d in ('thumb', 'index', 'middle', 'ring', 'pinky')]


def clasp():
    h = layout.clasp_hands()
    code, text, s = solve('clasp', pose(h, [{'grip': ['me.right', 'me.left'], 'style': 'interlaced', 'vary': FINGERS}]))
    hands = s['hands']
    for side in ('right', 'left'):
        pl = hands['me.' + side]['place']
        pl['forearm_toward'] = toward_elbow(side, pl['wrist'])
    return code, text, h, hands


if __name__ == '__main__':
    code, text, before, hands = clasp()
    print(code, '\n'.join(text.splitlines()[:8]))
    for side in ('right', 'left'):
        print(side, before['me.' + side]['place']['wrist'], '->', hands['me.' + side]['place']['wrist'])
    dump(os.path.join(OUT, 'clasp_hands.json'), hands)
    print('\n'.join(l for l in query(pose(hands)).splitlines()[:16]))
    still('clasp_side', hands)
    still('clasp_front', hands, FRONT)
