"""Per-frame joints of film.json for the launch film's code overlay, all from Handkit itself:
`handkit sample` gives the clip's joint values at a frame, `handkit geometry` the skeleton they
make (to 0.01 mm: `fk` prints whole millimetres, which would jitter on screen), projected through
the clip's camera. Writes joints.json:
  names   joint names, in `xy` order; bones  pairs of indices into names
  xy      per frame, x, y pixels of a 1920x1080 frame for every joint
  values  per frame, {digit: {joint: degrees}}"""
import json, math, os, subprocess, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
H = os.path.abspath(os.path.join(HERE, '..', '..', 'target', 'release', 'handkit.exe'))
CLIP = os.path.join(HERE, 'film.json')
c = json.load(open(CLIP))
cam, FRAMES = c['camera'], round(c['duration'] * c['fps'])
W, H_PX = c['size']
DIGITS = ('thumb', 'index', 'middle', 'ring', 'pinky')
FINGERS = DIGITS[1:]
NAMES = ['hand', 'thumb_cmc', 'thumb_mcp', 'thumb_ip', 'thumb.tip']
for d in FINGERS:
    NAMES += ['%s_mcp' % d, '%s_pip' % d, '%s_dip' % d, '%s.tip' % d]
I = {n: i for i, n in enumerate(NAMES)}
chain = lambda *ns: [[I[a], I[b]] for a, b in zip(ns, ns[1:])]
BONES = chain('hand', 'thumb_cmc', 'thumb_mcp', 'thumb_ip', 'thumb.tip')
for d in FINGERS:
    BONES += chain('hand', '%s_mcp' % d, '%s_pip' % d, '%s_dip' % d, '%s.tip' % d)
BONES += chain('index_mcp', 'middle_mcp', 'ring_mcp', 'pinky_mcp')


def sub(a, b): return [a[i] - b[i] for i in range(3)]
def dot(a, b): return sum(a[i] * b[i] for i in range(3))
def cross(a, b): return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
def unit(a):
    n = math.sqrt(dot(a, a))
    return [x / n for x in a]


P = cam['position']
F = unit(sub(cam['target'], P))
R = unit(cross(F, [0, 1, 0]))
U = cross(R, F)
T = math.tan(math.radians(cam['fov']) / 2)


def project(p):
    v = sub(p, P)
    z = dot(v, F)
    return [(0.5 + 0.5 * dot(v, R) / (z * T * W / H_PX)) * W, (0.5 - 0.5 * dot(v, U) / (z * T)) * H_PX]


def run(*a):
    return subprocess.run([H, *a], capture_output=True, text=True, check=True).stdout


xy, values = [], []
tmp = os.path.join(tempfile.gettempdir(), 'sign_frame.json')
for f in range(FRAMES):
    text = run('sample', CLIP, str(f / c['fps']))
    with open(tmp, 'w') as out:
        out.write(text)
    hand = json.loads(text[text.index('{'):])['hands']['right']
    frame = {d: {k: round(v, 1) for k, v in hand[d].items()} for d in DIGITS}
    frame['wrist'] = {k: round(hand.get(k, 0.0), 1) for k in ('twist', 'wrist_flex', 'wrist_dev', 'palm_cup')}
    values.append(frame)
    g = json.loads(run('geometry', tmp))
    start = {c['label'].split('.', 1)[1]: c['a'] for c in g['capsules'] if not c.get('arm')}
    site = {x['name'].split('.', 1)[1]: x['pos'] for x in g['sites']}
    pts = {'thumb_cmc': start['thumb.metacarpal'], 'thumb_mcp': start['thumb.proximal'], 'thumb_ip': start['thumb.distal'], 'thumb.tip': site['thumb.tip']}
    for d in FINGERS:
        pts.update({'%s_mcp' % d: start[d + '.proximal'], '%s_pip' % d: start[d + '.middle'], '%s_dip' % d: start[d + '.distal'], d + '.tip': site[d + '.tip']})
    roots = [start[d + '.metacarpal'] for d in FINGERS]
    pts['hand'] = [sum(r[i] for r in roots) / 4 for i in range(3)]
    xy.append([round(x, 1) for n in NAMES for x in project(pts[n])])
with open(os.path.join(HERE, 'joints.json'), 'w', newline='\n') as out:
    json.dump({'names': NAMES, 'bones': BONES, 'xy': xy, 'values': values}, out, separators=(',', ':'))
print(FRAMES, 'frames', len(NAMES), 'joints')
