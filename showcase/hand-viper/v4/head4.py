"""The tutorial's head (v1 of h3v): fists, palms to the viewer tilted up, the fingers in toward
each other, curled over; the index fingers across the front, tips meeting (the snout); each thumb
pressed up under its own index (the jaw, shut); the other fingers cleared of each other."""
from head3 import *

BASE = dict(curl=(70, 100, 60), index=(55, 70, 20), up=0.8, z=float(os.environ.get('DZ', '0.02')))
TILT = 0.4
INDEX = ['right.index.*', 'left.index.*']
CLOSED = [{'touch': ['right.index.tip', 'left.index.tip'], 'vary': INDEX},
          {'touch': ['right.thumb.pad', 'right.index.distal'], 'vary': ['right.thumb.*']},
          {'touch': ['left.thumb.pad', 'left.index.distal'], 'vary': ['left.thumb.*']},
          {'vary': ['%s.%s.*' % (s, d) for s in ('right', 'left') for d in ('middle', 'ring', 'pinky')]}]


def base():
    h = hands3(**BASE)
    for s in h:
        h[s]['place']['palm_facing'] = [0, TILT, 1]
    return h


def center(pose):
    src = os.path.join(OUT, '_c.json')
    dump(src, pose)
    t = run([H, 'query', src, '--json'])[1]
    q = json.loads(t[t.index('{'):])
    tips = list(q['fingertips_m'].values())
    return [sum(p[i] for p in tips) / len(tips) for i in range(3)]


if __name__ == '__main__':
    code, text, sol = solve('h4', {'version': 1, 'hands': base(), 'goals': CLOSED})
    print(code, [l.strip()[:120] for l in text.splitlines() if 'MISS' in l or 'penetr' in l])
    c = center({'version': 1, 'hands': sol['hands']})
    print('center', [round(x, 3) for x in c])
    cam = {'position': [c[0], c[1] + 0.02, c[2] + 0.42], 'target': [c[0], c[1] - 0.02, c[2]], 'fov': 26}
    still('h4', sol, cam=cam)
    still('h4_top', sol, cam={'position': [c[0], c[1] + 0.3, c[2] + 0.2], 'target': c, 'fov': 28})
