"""Shared bits for the camera-facing hand viper (the snake of the "Snake Hand Trick" tutorial):
both hands upright side by side, palms to the camera (the right hand on the viewer's left, so
both thumbs point to the middle). Middle, ring and little fingers curl forward and lock into
each other on top (the head; the right little finger and the left ring finger stand up as the
eyes), the index fingers curl in and meet in front (the snout and upper jaw) and the thumbs
point in underneath (the lower jaw): resting on the index fingers the mouth is shut, dropped
away it is open. The camera looks the snake in the face (+z toward it)."""
import json, os, subprocess, copy

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, '..', '..', '..'))
H = os.path.join(ROOT, 'target', 'release', 'handkit.exe')
STILL = os.path.join(ROOT, 'target', 'release', 'examples', 'still.exe')
A = os.path.abspath(os.path.join(ROOT, '..', '..', '..', 'assets')) if not os.path.exists(os.path.join(ROOT, 'assets', 'hand_R.glb')) else os.path.join(ROOT, 'assets')
OUT = os.path.join(HERE, 'out')
os.makedirs(OUT, exist_ok=True)
LOOK = {'style': 'realistic', 'skin': 'light', 'arm': {'sleeve': '#262a33', 'length': 0.12}}
CAM = {'position': [0.0, 0.11, 0.42], 'target': [0.0, 0.085, 0.0], 'fov': 28}
S = ('right', 'left')
X = {'right': -1, 'left': 1}   # the right hand on the viewer's left


def run(args):
    r = subprocess.run(args, capture_output=True, text=True)
    return r.returncode, r.stdout + r.stderr


def dump(path, obj):
    with open(path, 'w', newline='\n') as f:
        json.dump(obj, f, indent=1)


def solve(name, pose):
    src, dst = os.path.join(OUT, name + '.json'), os.path.join(OUT, name + '_sol.json')
    dump(src, pose)
    code, text = run([H, 'solve', src, '-o', dst])
    return code, text, (json.load(open(dst)) if os.path.exists(dst) else None)


def query(name, pose):
    src = os.path.join(OUT, name + '_q.json')
    dump(src, pose)
    return run([H, 'query', src])[1]


def still(name, pose, cam=None, size=520, bg='#e8e6e3'):
    p = copy.deepcopy(pose)
    p['look'] = LOOK
    p.pop('goals', None)
    src = os.path.join(OUT, name + '_still.json')
    dump(src, p)
    args = [STILL, src, os.path.join(OUT, name + '.png'), '--size', str(size), '--assets', A, '--bg', bg]
    if cam:
        args += ['--cam', ','.join(str(v) for v in cam)]
    return run(args)
