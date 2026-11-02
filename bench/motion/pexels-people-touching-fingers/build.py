"""Builds clip.json for seg01 (two index fingers almost touching).

Anchors are measured from the reference (index-tip position in frame fractions,
pointing angle in the image, finger values). The approach (0-1.9 s) is an
ease-out that is already moving at t=0, which Handkit's eases cannot express,
so it is written as linear keys on a dense grid sampled from a smooth (PCHIP)
curve through the anchors. The hold and the final pull-back use natural eases.

usage: python build.py [idle_amount]
"""
import json, math, subprocess, sys
from scipy.interpolate import PchipInterpolator

H = "C:/Users/Rajat/Documents/3D Accurate Hands/target/release/handkit.exe"
CAM = {"position": [0, 0, 1.04], "target": [0, 0, 0], "fov": 70}
TAN = math.tan(math.radians(CAM["fov"] / 2)); ASP = 1080 / 1920; D = CAM["position"][2]


def unproject(fx, fy, z):
    dist = D - z
    return [(fx - 0.5) * 2 * TAN * ASP * dist, (0.5 - fy) * 2 * TAN * dist, z]


def project(p):
    dist = D - p[2]
    return (0.5 + p[0] / (2 * TAN * ASP * dist), 0.5 - p[1] / (2 * TAN * dist))


# finger values: thumb [cmc_flex, cmc_abd, cmc_rot, mcp_flex, ip_flex], others [mcp_flex, pip_flex]
TOP_F_EARLY = {"thumb": [0, 20, 10, 10, 10], "index": [10, 15], "middle": [14, 24], "ring": [18, 30], "pinky": [22, 32]}
TOP_F = {"thumb": [0, 22, 10, 12, 12], "index": [0, 5], "middle": [25, 45], "ring": [35, 55], "pinky": [40, 55]}
BOT_F_EARLY = {"thumb": [15, 30, 15, 15, 15], "index": [8, 15], "middle": [45, 35], "ring": [55, 50], "pinky": [60, 55]}
BOT_F = {"thumb": [12, 40, 25, 5, 5], "index": [0, 3], "middle": [30, 8], "ring": [48, 38], "pinky": [55, 50]}


PALM_Z = {"right": 0.15, "left": -0.2}
# knuckle spread (mcp_abd): the reference keeps the curled fingers close together
ABD = {"right": {"index": -4, "middle": 0, "ring": -5, "pinky": -8}, "left": {"index": 0, "middle": 0, "ring": -4, "pinky": -6}}


def mixf(a, b, s):
    return {k: [a[k][i] * (1 - s) + b[k][i] * s for i in range(len(a[k]))] for k in a}


# anchors: t, tip fx, tip fy, tip depth z, image angle of the fingers (deg, y up),
# fingers z component, wrist_flex, wrist_dev, finger values
top = [  # right hand, from the top right
    (0.00, 1.02, 0.565, 0.21, -95, -0.20, 10, 0, TOP_F_EARLY),
    (0.10, 0.93, 0.552, 0.21, -97, -0.22, 10, 0, TOP_F_EARLY),
    (0.20, 0.855, 0.540, 0.21, -102, -0.25, 12, 0, mixf(TOP_F_EARLY, TOP_F, 0.2)),
    (0.30, 0.785, 0.522, 0.21, -110, -0.28, 15, 0, mixf(TOP_F_EARLY, TOP_F, 0.4)),
    (0.45, 0.715, 0.500, 0.21, -118, -0.30, 18, 0, mixf(TOP_F_EARLY, TOP_F, 0.7)),
    (0.65, 0.638, 0.473, 0.21, -125, -0.30, 20, 0, TOP_F),
    (0.85, 0.575, 0.457, 0.21, -129, -0.30, 20, 0, TOP_F),
    (1.05, 0.550, 0.455, 0.21, -131, -0.30, 20, 0, TOP_F),
    (1.30, 0.531, 0.457, 0.21, -132, -0.30, 20, 0, TOP_F),
    (1.55, 0.514, 0.457, 0.21, -133, -0.30, 20, 0, TOP_F),
    (1.90, 0.502, 0.455, 0.21, -133, -0.30, 20, 0, TOP_F),
]
bot = [  # left hand, from the bottom left
    (0.00, 0.520, 0.830, -0.01, 10, -0.20, 5, 0, BOT_F_EARLY),
    (0.10, 0.522, 0.770, -0.01, 11, -0.20, 4, 0, BOT_F_EARLY),
    (0.20, 0.520, 0.715, -0.01, 13, -0.22, 3, 0, BOT_F_EARLY),
    (0.30, 0.505, 0.677, -0.01, 16, -0.24, 2, 0, mixf(BOT_F_EARLY, BOT_F, 0.15)),
    (0.45, 0.500, 0.630, -0.01, 24, -0.32, 0, 0, mixf(BOT_F_EARLY, BOT_F, 0.35)),
    (0.65, 0.484, 0.570, -0.01, 36, -0.38, -6, 0, mixf(BOT_F_EARLY, BOT_F, 0.65)),
    (0.85, 0.474, 0.528, -0.01, 42, -0.42, -11, 0, mixf(BOT_F_EARLY, BOT_F, 0.85)),
    (1.05, 0.471, 0.498, -0.01, 44, -0.44, -14, 0, BOT_F),
    (1.30, 0.467, 0.478, -0.01, 45, -0.45, -15, 0, BOT_F),
    (1.55, 0.466, 0.468, -0.01, 45, -0.45, -15, 0, BOT_F),
    (1.90, 0.467, 0.465, -0.01, 45, -0.45, -15, 0, BOT_F),
]


def interp(anchors, t):
    ts = [a[0] for a in anchors]
    out = [float(PchipInterpolator(ts, [a[j] for a in anchors])(t)) for j in range(1, 8)]
    i = max(k for k in range(len(ts)) if ts[k] <= t + 1e-9)
    if i == len(ts) - 1:
        f = anchors[-1][8]
    else:
        s = (t - ts[i]) / (ts[i + 1] - ts[i])
        f = mixf(anchors[i][8], anchors[i + 1][8], s)
    return out + [f]


def hand_json(side, st, wrist):
    fx, fy, z, ang, fz, flex, dev, f = st
    a = math.radians(ang)
    fingers = [math.cos(a), math.sin(a), fz]
    pa = a + (math.pi / 2 if side == "right" else -math.pi / 2)
    palm = [math.cos(pa), math.sin(pa), PALM_Z[side]]
    r = lambda v: round(v, 4)
    h = {"place": {"wrist": [r(v) for v in wrist], "palm_facing": [r(v) for v in palm], "fingers_toward": [r(v) for v in fingers]},
         "wrist_flex": r(flex), "wrist_dev": r(dev), "palm_cup": 6,
         "thumb": dict(zip(["cmc_flex", "cmc_abd", "cmc_rot", "mcp_flex", "ip_flex"], [r(v) for v in f["thumb"]]))}
    for k in ["index", "middle", "ring", "pinky"]:
        h[k] = {"mcp_flex": r(f[k][0]), "pip_flex": r(f[k][1]), "mcp_abd": ABD[side][k]}
    return h


def fk_points(pose):
    open("work/_fk.json", "w").write(json.dumps(pose))
    out = subprocess.run([H, "fk", "work/_fk.json"], capture_output=True, text=True).stdout
    pts = {}
    for line in out.splitlines():
        line = line.replace("site ", "")
        if ":" in line and "[" in line:
            name = line.split(":")[0].strip()
            pts[name] = [float(v) for v in line.split("[")[1].split("]")[0].split()]
    return pts


def solve_state(side, st):
    """Wrist position that puts the index tip at the target frame point; also the projected elbow."""
    h = hand_json(side, st, [0, 0, 0])
    pts = fk_points({"version": 1, "hands": {side: h}})
    off = pts[f"{side}.index.tip"]
    tip = unproject(st[0], st[1], st[2])
    wrist = [tip[i] - off[i] for i in range(3)]
    elbow = [pts[f"{side}.forearm"][i] + wrist[i] for i in range(3)]
    return hand_json(side, st, wrist), project(elbow)


keys, report = [], []
# approach: linear keys on a dense grid (the last one eases to a stop)
grid = [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2, 1.35, 1.5, 1.65, 1.8, 1.95]
for t in grid:
    tt = min(t, 1.9)
    hr, er = solve_state("right", interp(top, tt))
    hl, el = solve_state("left", interp(bot, tt))
    k = {"t": t, "hands": {"right": hr, "left": hl}}
    if t > 0:
        k["ease"] = "linear" if t < 1.9 else "smooth"
    keys.append(k)
    report.append((t, er, el))

# hold: small drifts measured by template tracking, as index-tip offsets from the settled point.
# (t, move, top dfx, top dfy, bottom dfx, bottom dfy)
hold = [
    (2.65, 0.5, +0.004, +0.002, +0.006, +0.002),
    (3.2, 0.5, +0.000, -0.001, +0.004, +0.002),
    (4.1, 0.9, -0.006, -0.002, +0.002, +0.005),
    (4.6, 0.5, -0.006, -0.001, +0.006, +0.002),
    (5.1, 0.5, -0.009, -0.002, +0.008, +0.001),
    (6.1, 1.0, -0.008, -0.004, -0.003, -0.002),
    (6.6, 0.5, -0.009, -0.003, -0.001, +0.000),
    (7.0, 0.4, -0.013, -0.004, +0.000, +0.002),
    (7.6, 0.6, -0.009, -0.004, +0.006, +0.001),
    (8.45, 0.8, -0.008, -0.011, +0.008, +0.001),
    # the pull-back that starts at the very end of the clip (arrives after it ends)
    (9.2, 0.7, +0.079, +0.030, +0.025, +0.060),
]
for (t, mv, tdx, tdy, bdx, bdy) in hold:
    st = list(top[-1][1:]); st[0] += tdx; st[1] += tdy
    sb = list(bot[-1][1:]); sb[0] += bdx; sb[1] += bdy
    hr, er = solve_state("right", st)
    hl, el = solve_state("left", sb)
    keys.append({"t": t, "move": mv, "hands": {"right": hr, "left": hl}})
    report.append((t, er, el))

clip = {"version": 1, "name": "pexels-people-touching-fingers seg01",
        "description": "Two arms reach into a spotlight from the top right and bottom left; the index fingers end almost touching (Creation of Adam), hold with small drifts, and begin to pull back.",
        "fps": 30, "duration": 8.9, "size": [1080, 1920], "camera": CAM,
        "follow": 0,
        "idle": {"amount": float(sys.argv[1]) if len(sys.argv) > 1 else 0.6, "seed": 3},
        "keys": keys}
json.dump(clip, open("clip.json", "w"), indent=1)
for t, er, el in report:
    print(f"t {t:5.2f}  right elbow ({er[0]:.2f},{er[1]:.2f})  left elbow ({el[0]:.2f},{el[1]:.2f})")
