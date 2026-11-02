"""The hand viper as the tutorial makes it (youtube uroglg7zo8E), facing the camera: two fists
side by side, palms toward each other and the thumb sides toward the viewer; middle, ring and
little fingers interlaced over the other hand's back (the head, with the right little finger
and the left ring finger as the eyes on top); the index fingers curled across the front, tips
meeting (the snout); the thumbs bent up underneath, against the index fingers (the jaw, shut).
The camera is +z; the person is at -z, so the right hand is on screen right of the snake...
seen from the front, i.e. on the viewer's left? No: the viewer faces the snake, the snake's
left eye (right little finger) is on the viewer's right, so the right hand is at +x."""
import json, os, subprocess, sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
H = os.path.join(ROOT, 'target', 'release', 'handkit.exe')
STILL = os.path.join(ROOT, 'target', 'release', 'examples', 'still.exe')
A = os.path.join(ROOT, 'assets')
HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, 'out')
FRONT = {'position': [0.0, 0.07, 0.36], 'target': [0.0, 0.09, 0.0], 'fov': 28}
LOOK = {'style': 'realistic', 'skin': 'medium', 'arm': {'sleeve': '#262a33', 'length': 0.12}}


def run(args):
    r = subprocess.run(args, capture_output=True, text=True)
    return r.returncode, r.stdout + r.stderr


def dump(path, obj):
    with open(path, 'w', newline='\n') as f:
        json.dump(obj, f, indent=1)


def hands(half=0.034, dz=0.006, tilt=0.0):
    """Both hands upright, palms toward each other, `half` m from the middle; the right hand
    `dz` m nearer the viewer than the left so the fingers can alternate."""
    h = {}
    for side, s in (('right', 1), ('left', -1)):
        h[side] = {'place': {'wrist': [s * half, 0.0, s * dz], 'palm_facing': [-s, 0, tilt], 'fingers_toward': [0, 1, 0],
                             'forearm_toward': [s * 0.15, -1, -0.45]},
                   'index': {'mcp_abd': 0, 'mcp_flex': 70, 'pip_flex': 80, 'dip_flex': 20},
                   'thumb': {'cmc_flex': 10, 'cmc_abd': 20, 'cmc_rot': 30, 'mcp_flex': 20, 'ip_flex': 30}}
    return h


def solve(name, pose):
    src, dst = os.path.join(OUT, name + '.json'), os.path.join(OUT, name + '_sol.json')
    dump(src, pose)
    code, text = run([H, 'solve', src, '-o', dst])
    return code, text, (json.load(open(dst)) if os.path.exists(dst) else None)


def still(name, pose, cam=None, size=600):
    p = dict(pose, look=LOOK)
    p.pop('goals', None)
    src = os.path.join(OUT, name + '_still.json')
    dump(src, p)
    c = cam or FRONT
    args = [STILL, src, os.path.join(OUT, name + '.png'), '--size', str(size), '--assets', A, '--bg', '#c9ccd2',
            '--cam', ','.join(str(x) for x in c['position'] + c['target'] + [c['fov']])]
    return run(args)


INTERLACE = {'grip': ['right', 'left'], 'style': 'interlaced',
             'vary': ['%s.%s.*' % (s, d) for s in ('right', 'left') for d in ('middle', 'ring', 'pinky')]}

if __name__ == '__main__':
    pose = {'version': 1, 'hands': hands(), 'camera': FRONT, 'goals': [INTERLACE]}
    code, text, sol = solve('head1', pose)
    print(code, [l.strip()[:160] for l in text.splitlines() if 'MISS' in l or ' ok ' in l or 'penetr' in l])
    still('head1', sol)
    still('head1_top', sol, cam={'position': [0.0, 0.32, 0.12], 'target': [0.0, 0.09, 0.0], 'fov': 30})
