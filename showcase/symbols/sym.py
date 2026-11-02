"""Generalization checks for the viper's tools: other two-hand symbols built the same way
(placed hands, grips and touches that vary only the digits), solved, checked and rendered."""
import json, os, subprocess, sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
H = os.path.join(ROOT, 'target', 'release', 'handkit.exe')
STILL = [os.path.join(ROOT, 'target', 'release', 'examples', 'still.exe')]
A = os.path.join(ROOT, 'assets')
HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, 'out')
os.makedirs(OUT, exist_ok=True)
LOOK = {'style': 'realistic', 'skin': 'light', 'arm': {'sleeve': '#262a33', 'length': 0.12}}


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


def still(name, pose, size=560, cam=None):
    src = os.path.join(OUT, name + '_still.json')
    p = dict(pose)
    p['look'] = LOOK
    dump(src, p)
    args = STILL + [src, os.path.join(OUT, name + '.png'), '--size', str(size), '--assets', A, '--bg', '#1b1d22']
    if cam:
        args += ['--cam', ','.join(str(x) for x in cam)]
    return run(args)
