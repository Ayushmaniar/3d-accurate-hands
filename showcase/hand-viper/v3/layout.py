"""Where the hands and elbows go for the clasp: the left hand where Handkit's interlaced grip puts
a follower (45 mm out from the right palm, 10 mm toward its thumb, fingers parallel), and a search
over elbow heights and spread for crossed forearms that clear each other within the wrist limits."""
import itertools, math, re
from hk3 import *
import base

OFFSET = (0.010, 0.0, 0.045)   # the follower's wrist in the leader's frame (grip.rs, interlaced)


def frame_of(pl):
    y = [pl['fingers_toward'][i] for i in range(3)]
    n = math.sqrt(sum(v * v for v in y)); y = [v / n for v in y]
    z = pl['palm_facing']
    d = sum(z[i] * y[i] for i in range(3)); z = [z[i] - d * y[i] for i in range(3)]
    n = math.sqrt(sum(v * v for v in z)); z = [v / n for v in z]
    x = [y[1] * z[2] - y[2] * z[1], y[2] * z[0] - y[0] * z[2], y[0] * z[1] - y[1] * z[0]]
    return x, y, z


def clasp_hands(tilt=0.0, lift=0.0):
    """Right hand on top of the cross, the left one where the grip puts it."""
    h = base.hands()
    c, s = math.cos(math.radians(tilt)), math.sin(math.radians(tilt))
    rw = [0.0, lift, -0.0225]
    h['me.right']['place'] = place('right', rw, [0, 0, 1], [c, -s, 0])
    x, y, z = frame_of(h['me.right']['place'])
    lw = [rw[i] + OFFSET[0] * x[i] + OFFSET[1] * y[i] + OFFSET[2] * z[i] for i in range(3)]
    h['me.left']['place'] = place('left', lw, [-v for v in z], y)
    return h


def score(h):
    out = query(pose(h))
    fa = [float(m) for m in re.findall(r'me\.right\.forearm ~ me\.left\.forearm: (-[\d.]+) mm', out)]
    lim = out.split('limit violations:')[1].splitlines()[0].strip() != 'none'
    return (min(fa) if fa else 0.0), lim


if __name__ == '__main__':
    res = []
    for ry, ly, ez, tilt in itertools.product([0.08, 0.14, 0.20], [-0.12, -0.18, -0.24], [0.08, 0.15, 0.22], [0, 15]):
        ELBOW['right'] = [-0.22, ry, ez]; ELBOW['left'] = [-0.22, ly, -ez]
        d, lim = score(clasp_hands(tilt))
        res.append((lim, -d, ry, ly, ez, tilt))
    res.sort()
    for r in res[:10]:
        print(r)
