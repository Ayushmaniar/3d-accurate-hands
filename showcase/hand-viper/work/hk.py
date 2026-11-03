"""Small helpers: write a pose, solve it, preview it as a one-key clip."""
import json, subprocess, os, sys, copy

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
H = os.path.join(ROOT, 'target', 'release', 'handkit.exe')
P = os.path.join(ROOT, 'starters', 'fframes-clip', 'target', 'release', 'fframes-clip.exe')
A = os.path.join(ROOT, 'assets')
WORK = os.path.dirname(os.path.abspath(__file__))
BG = '#1b1d22'
SPOT = '0.5,0.45,0.42,#3a3f48'

CAM = {'position': [0.0, 0.25, -0.40], 'target': [0, 0.11, 0], 'fov': 30}
LOOK = {'style': 'realistic', 'skin': 'light', 'motion_blur': 3, 'shutter': 120,
        'arm': {'sleeve': '#262a33', 'length': 0.12}}


def run(args, check=True):
    r = subprocess.run(args, capture_output=True, text=True)
    return r.returncode, r.stdout + r.stderr


def write(path, obj):
    with open(path, 'w') as f:
        json.dump(obj, f, indent=1)


def pose_file(name, hands, goals=None, camera=None):
    p = {'version': 1, 'hands': hands, 'goals': goals or [], 'camera': camera or CAM,
         'size': [1920, 1080]}
    path = os.path.join(WORK, name + '.json')
    write(path, p)
    return path


def solve(name, hands, goals, camera=None):
    src = pose_file(name, hands, goals, camera)
    out = os.path.join(WORK, name + '_sol.json')
    code, txt = run([H, 'solve', src, '-o', out])
    sol = json.load(open(out))
    return code, txt, sol


def query(path):
    return run([H, 'query', path])[1]


def preview(name, hands, camera=None, out=None, width=640):
    clip = {'version': 1, 'fps': 30, 'duration': 0.5, 'size': [1920, 1080],
            'camera': camera or CAM, 'look': LOOK, 'idle': {'amount': 0}, 'collide': False,
            'keys': [{'t': 0, 'hands': hands}]}
    cp = os.path.join(WORK, name + '_clip.json')
    write(cp, clip)
    out = out or os.path.join(WORK, name + '.png')
    code, txt = run([P, '--clip', cp, '--assets', A, 'strip', '0.1s', '-n', '1', '--columns', '1',
                     '--width', str(width), '--background', BG, '--spotlight', SPOT, '-o', out])
    return out, txt
