"""The face of the snake, mouth shut, from the hand_viper gesture's clasp (palms together, middle,
ring and little fingers interlaced over the backs; the head points +x): the index fingers curl
in and meet tip to tip across the front (the snout and upper jaw), the thumbs meet tip to tip
underneath (the lower jaw) and each index rests on its thumb, so the mouth is shut."""
from fk import *

G = json.load(open(os.path.join(ROOT, 'crates', 'handkit-core', 'gestures', 'hand_viper.json')))
JAW = ['%s.%s.*' % (s, d) for s in S for d in ('index', 'thumb')]
# The thumbs come up from underneath, their tips against the curled index fingers.
SHUT = [
    {'touch': ['right.index.tip', 'left.index.tip'], 'vary': JAW},
    {'touch': ['right.thumb.tip', 'left.thumb.tip'], 'vary': JAW},
    {'touch': ['right.index.pad', 'right.thumb.nail'], 'vary': JAW},
    {'touch': ['left.index.pad', 'left.thumb.nail'], 'vary': JAW},
]
LOCK = ['%s.%s.*' % (s, d) for s in S for d in ('middle', 'ring', 'pinky')]


# The eyes: the right little finger and the left ring finger, whose bent middle joints stand up
# as the two highest knuckles on top of the head.
EYES = (('right', 'pinky'), ('left', 'ring'))


def hooked(hands, lift=8, sink=0):
    """The interlaced fingers hooked tight (tips tucked out of sight), the two eye fingers bent
    `lift` degrees less at the knuckle and the others `sink` more, so the eyes' middle joints
    stand above the rest."""
    out = copy.deepcopy(hands)
    for s in S:
        for d in ('middle', 'ring', 'pinky'):
            f = out[s][d]
            eye = (s, d) in EYES
            f.update(mcp_flex=max(-30, min(90, f['mcp_flex'] + (-lift if eye else sink))), pip_flex=100 if eye else 105, dip_flex=70)
    return out


# Looking the snake in the face: in front of the snout (+x), level with it.
FACE = [0.29, 0.17, 0.0, 0.07, 0.09, 0.0, 30]
FACE_34 = [0.36, 0.17, 0.16, 0.06, 0.085, 0.0, 26]

# The hooked fingers get out of each other by spreading, rolling and bending at the knuckle;
# their middle and last joints stay hooked.
SPREAD = ['%s.*.mcp_abd' % s for s in S] + ['%s.*.mcp_roll' % s for s in S] + ['%s.%s.mcp_flex' % (s, d) for s in S for d in ('middle', 'ring', 'pinky')]

if __name__ == '__main__':
    pose = {'version': 1, 'hands': hooked(G['hands']), 'camera': G['camera'], 'goals': SHUT + [{'vary': SPREAD}]}
    code, text, sol = solve('head', pose)
    print(code, '\n'.join(l for l in text.splitlines() if 'touch' in l or 'penetr' in l))
    if sol:
        sol.pop('goals', None)
        dump(os.path.join(HERE, 'face.json'), sol)
        still('head', sol, cam=FACE)
        still('head_34', sol, cam=FACE_34)
