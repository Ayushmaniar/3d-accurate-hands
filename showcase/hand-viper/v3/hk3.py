"""Shared bits for the hand viper, third pass.

The snake frame: the head points +x (screen right from the side camera), up is +y. The wrists
cross: the person faces +x with the right arm on +z, so the right hand ends up on -z, palm
facing +z, and the left hand on +z, palm facing -z. Palms together like that put both thumbs
underneath (the jaw), the index fingers just above them (the snout) and the pinkies on top.
"""
import json, os, subprocess, copy

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
H = os.path.join(ROOT, 'target', 'release', 'handkit.exe')
P = os.path.join(ROOT, 'starters', 'fframes-clip', 'target', 'release', 'fframes-clip.exe')
A = os.path.join(ROOT, 'assets')
HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, 'out')
os.makedirs(OUT, exist_ok=True)
FF = os.path.expanduser('~/tools/ffmpeg-n9.0-latest-win64-gpl-shared-9.0/bin')
os.environ['PATH'] = FF + os.pathsep + os.environ['PATH']

BG, SPOT = '#1b1d22', '0.5,0.45,0.42,#3a3f48'
SIDE = {'position': [0.02, 0.07, 0.42], 'target': [0.02, -0.005, 0.0], 'fov': 30}
FRONT = {'position': [0.42, 0.10, 0.10], 'target': [0.02, -0.005, 0.0], 'fov': 30}
LOOK = {'style': 'realistic', 'skin': 'light', 'motion_blur': 3,
        'arm': {'sleeve': '#262a33', 'length': 0.12}}
# Elbows: the right arm comes from the person's right (+z) and behind (-x), crossing to -z.
# The wrists cross one over the other (right on top), so the forearms clear each other.
ELBOW = {'right': [-0.24, 0.06, 0.15], 'left': [-0.24, -0.14, -0.15]}


def run(args):
    r = subprocess.run(args, capture_output=True, text=True)
    return r.returncode, r.stdout + r.stderr


def dump(path, obj):
    with open(path, 'w', newline='\n') as f:
        json.dump(obj, f, indent=1)


def toward_elbow(side, wrist):
    e = ELBOW[side]
    return [e[i] - wrist[i] for i in range(3)]


def place(side, wrist, palm, fingers):
    return {'wrist': wrist, 'palm_facing': palm, 'fingers_toward': fingers,
            'forearm_toward': toward_elbow(side, wrist)}


def pose(hands, goals=None, camera=None):
    p = {'version': 1, 'people': {'me': {'look': {'skin': 'light'}}}, 'hands': hands,
         'camera': camera or SIDE, 'size': [1920, 1080]}
    if goals:
        p['goals'] = goals
    return p


def solve(name, p):
    src, dst = os.path.join(OUT, name + '.json'), os.path.join(OUT, name + '_sol.json')
    dump(src, p)
    code, text = run([H, 'solve', src, '-o', dst])
    return code, text, json.load(open(dst)) if os.path.exists(dst) else None


def query(p, name='q'):
    src = os.path.join(OUT, name + '.json')
    dump(src, p)
    return run([H, 'query', src])[1]


def still(name, hands, camera=None, width=640):
    """One frame of a one-key clip, written to out/<name>.png."""
    clip = {'version': 1, 'fps': 30, 'duration': 0.2, 'idle': 0, 'collide': False, 'camera': camera or SIDE, 'look': LOOK,
            'keys': [{'t': 0, 'hands': copy.deepcopy(hands)}]}
    path = os.path.join(OUT, name + '_clip.json')
    dump(path, clip)
    code, text = run([P, '--clip', path, '--assets', A, '--background', BG, '--spotlight', SPOT,
                      'frame', '0.1s', '-o', OUT])
    src = os.path.join(OUT, '0.1s.png')
    dst = os.path.join(OUT, name + '.png')
    if os.path.exists(src):
        from PIL import Image
        im = Image.open(src)
        im.resize((width, width * im.size[1] // im.size[0])).save(dst)
        os.remove(src)
    return code, text
