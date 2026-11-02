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
    """The interlaced fingers hooked over each other's knuckles, lying flat so the top of the head
    is solid (tips tucked out of sight); the two eye fingers bent
    `lift` degrees less at the knuckle and the others `sink` more, so the eyes' middle joints
    stand above the rest."""
    out = copy.deepcopy(hands)
    for s in S:
        for d in ('middle', 'ring', 'pinky'):
            f = out[s][d]
            eye = (s, d) in EYES
            f.update(mcp_flex=max(-30, min(90, f['mcp_flex'] + (-lift if eye else sink))), pip_flex=90 if eye else 95, dip_flex=30)
    return out


# Looking the snake in the face: in front of the snout (+x), level with it.
FACE = [0.29, 0.17, 0.0, 0.07, 0.09, 0.0, 30]
FACE_34 = [0.36, 0.17, 0.16, 0.06, 0.085, 0.0, 26]

# The right ring finger is the longest on top, so its knuckle would stand highest: it and the
# right middle finger bend further down at the knuckle (held there) so the two eyes top the head.
SINK = {('right', 'ring'): 25, ('right', 'middle'): 10}


def with_eyes(hands):
    out = hooked(hands, lift=15)
    for (s, d), deg in SINK.items():
        out[s][d]['mcp_flex'] = min(90, out[s][d]['mcp_flex'] + deg)
    return out


# The hooked fingers get out of each other by spreading and rolling; the ones that are neither
# eyes nor sunk also bend at the knuckle, and the sunk ones at their middle and last joints.
FREE = [(s, d) for s in S for d in ('middle', 'ring', 'pinky') if (s, d) not in EYES and (s, d) not in SINK]
SPREAD = (['%s.*.mcp_abd' % s for s in S] + ['%s.*.mcp_roll' % s for s in S] + ['%s.%s.mcp_flex' % sd for sd in FREE]
          + ['%s.%s.%s' % (s, d, j) for (s, d) in SINK for j in ('pip_flex', 'dip_flex')])

if __name__ == '__main__':
    pose = {'version': 1, 'hands': with_eyes(G['hands']), 'camera': G['camera'], 'goals': SHUT + [{'vary': SPREAD}]}
    code, text, sol = solve('head', pose)
    print(code, '\n'.join(l for l in text.splitlines() if 'touch' in l or 'penetr' in l))
    if sol:
        sol.pop('goals', None)
        dump(os.path.join(HERE, 'face.json'), sol)
        still('head', sol, cam=FACE)
        still('head_34', sol, cam=FACE_34)
