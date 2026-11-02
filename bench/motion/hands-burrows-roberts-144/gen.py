"""Builds clip.json. Wrist positions are given as 4:3 frame fractions on a depth plane or the lap."""
import json, math
import numpy as np

CAM = {"position": [0, 0.55, 1.0], "target": [0, 0.02, 0.08], "fov": 22}
P = np.array(CAM["position"], float); T = np.array(CAM["target"], float)
f = T - P; f /= np.linalg.norm(f)
r = np.cross(f, [0, 1, 0]); r /= np.linalg.norm(r)
u = np.cross(r, f)
th = math.tan(math.radians(CAM["fov"]) / 2); tw = th * 4 / 3
LAP_N = np.array([0, 0.45, 0.9]) / np.linalg.norm([0, 0.45, 0.9])
LAP_P = np.array([0, -0.05, 0.08])

def ray(fx, fy):
    return f + (2 * fx - 1) * tw * r - (2 * fy - 1) * th * u

def at_z(fx, fy, z):
    d = ray(fx, fy); s = (z - P[2]) / d[2]; return P + s * d

def on_lap(fx, fy, lift=0.028):
    d = ray(fx, fy); s = ((LAP_P - P) @ LAP_N) / (d @ LAP_N)
    return P + s * d + LAP_N * lift

ELBOW = {"right": ("edge", 0.15, -0.08), "left": ("edge", 0.85, -0.08)}

def R3(v): return [round(float(x), 3) for x in v]

# finger shapes
SH = {
 "relaxed": {"palm_cup": 6, "thumb": {"cmc_abd": 25, "cmc_flex": 5, "cmc_rot": 15, "ip_flex": 15, "mcp_flex": 10},
   "index": {"mcp_abd": 0, "mcp_flex": 15, "pip_flex": 22}, "middle": {"mcp_abd": 0, "mcp_flex": 20, "pip_flex": 30},
   "ring": {"mcp_abd": 0, "mcp_flex": 25, "pip_flex": 36}, "pinky": {"mcp_abd": 2, "mcp_flex": 30, "pip_flex": 40}},
 "loose": {"palm_cup": 8, "thumb": {"cmc_abd": 30, "cmc_flex": 8, "cmc_rot": 20, "ip_flex": 20, "mcp_flex": 15},
   "index": {"mcp_abd": 0, "mcp_flex": 25, "pip_flex": 40}, "middle": {"mcp_abd": 0, "mcp_flex": 30, "pip_flex": 45},
   "ring": {"mcp_abd": 0, "mcp_flex": 35, "pip_flex": 50}, "pinky": {"mcp_abd": 2, "mcp_flex": 40, "pip_flex": 55}},
 "curled": {"palm_cup": 12, "thumb": {"cmc_abd": 35, "cmc_flex": 15, "cmc_rot": 30, "ip_flex": 30, "mcp_flex": 20},
   "index": {"mcp_abd": 0, "mcp_flex": 45, "pip_flex": 65}, "middle": {"mcp_abd": 0, "mcp_flex": 50, "pip_flex": 70},
   "ring": {"mcp_abd": 0, "mcp_flex": 55, "pip_flex": 72}, "pinky": {"mcp_abd": 2, "mcp_flex": 60, "pip_flex": 75}},
 "flat": {"palm_cup": 2, "thumb": {"cmc_abd": 12, "cmc_flex": 0, "cmc_rot": 10, "ip_flex": 8, "mcp_flex": 5},
   "index": {"mcp_abd": 3, "mcp_flex": 8, "pip_flex": 10}, "middle": {"mcp_abd": 0, "mcp_flex": 10, "pip_flex": 12},
   "ring": {"mcp_abd": 2, "mcp_flex": 12, "pip_flex": 14}, "pinky": {"mcp_abd": 6, "mcp_flex": 14, "pip_flex": 16}},
 "spread": {"palm_cup": 4, "thumb": {"cmc_abd": 20, "cmc_flex": -10, "cmc_rot": 10, "ip_flex": 10, "mcp_flex": 5},
   "index": {"mcp_abd": 10, "mcp_flex": 15, "pip_flex": 20}, "middle": {"mcp_abd": 2, "mcp_flex": 18, "pip_flex": 22},
   "ring": {"mcp_abd": 8, "mcp_flex": 20, "pip_flex": 25}, "pinky": {"mcp_abd": 15, "mcp_flex": 22, "pip_flex": 28}},
 "claw": {"palm_cup": 10, "thumb": {"cmc_abd": 30, "cmc_flex": 0, "cmc_rot": 20, "ip_flex": 30, "mcp_flex": 15},
   "index": {"mcp_abd": 8, "mcp_flex": 25, "pip_flex": 55}, "middle": {"mcp_abd": 0, "mcp_flex": 30, "pip_flex": 60},
   "ring": {"mcp_abd": 8, "mcp_flex": 35, "pip_flex": 62}, "pinky": {"mcp_abd": 15, "mcp_flex": 40, "pip_flex": 65}},
 "wrap": {"palm_cup": 8, "thumb": {"cmc_abd": 15, "cmc_flex": 0, "cmc_rot": 10, "ip_flex": 10, "mcp_flex": 10},
   "index": {"mcp_abd": 0, "mcp_flex": 25, "pip_flex": 30}, "middle": {"mcp_abd": 0, "mcp_flex": 28, "pip_flex": 32},
   "ring": {"mcp_abd": 0, "mcp_flex": 32, "pip_flex": 36}, "pinky": {"mcp_abd": 3, "mcp_flex": 36, "pip_flex": 40}},
}

def edge_elbow(w, fx, fy, L=0.27):
    """A point on the camera ray through frame fraction (fx, fy) at distance L from the wrist (the deeper one)."""
    d = ray(fx, fy); d = d / np.linalg.norm(d); w = np.array(w, float)
    b = d @ (P - w); c = (P - w) @ (P - w) - L * L; disc = b * b - c
    if disc < 0: return P + d * (-b)
    return P + d * (-b + math.sqrt(disc))


def hand(side, wrist, palm, fingers, shape="relaxed", elbow=None, tw=None, **extra):
    w = np.array(wrist, float)
    if elbow is None: elbow = ELBOW[side]
    if False: pass
    elif isinstance(elbow, tuple) and elbow[0] == "edge": e = edge_elbow(w, elbow[1], elbow[2])
    else: e = np.array(elbow, float)
    h = {"place": {"wrist": R3(w), "palm_facing": R3(palm), "fingers_toward": R3(fingers), "forearm_toward": R3(e - w)}}
    h.update(json.loads(json.dumps(SH[shape])))
    for k, v in extra.items():
        if isinstance(v, dict): h.setdefault(k, {}).update(v)
        else: h[k] = v
    return h

DOWN_LAP = np.array([0, -0.9, 0.45]); INTO_LAP = -LAP_N

def lapdir(xbias):
    v = DOWN_LAP / np.linalg.norm(DOWN_LAP) + np.array([xbias, 0, 0]); return v

import subprocess, os
HK = os.path.abspath("../../../target/release/handkit.exe")
FING = ["%s.%s.*" % (sd, d) for sd in ("right", "left") for d in ("index", "middle", "ring", "pinky", "thumb")]
def solve_pair(name, rh, lh, extra_vary=("left.place.position",), goals=None):
    """Both hands in one pose file; the solver clears penetrations (fingers + extra_vary) and meets goals."""
    os.makedirs("poses", exist_ok=True)
    g = goals or []
    g = g + [{"vary": FING + list(extra_vary)}]
    path = "poses/%s.json" % name
    json.dump({"version": 1, "hands": {"right": rh, "left": lh}, "goals": g, "camera": CAM}, open(path, "w"), indent=1)
    out = subprocess.run([HK, "solve", path], capture_output=True, text=True)
    d = json.load(open(path))
    q = subprocess.run([HK, "query", path], capture_output=True, text=True).stdout
    pen = q.split("penetrations")[1].splitlines()[0]
    print("solve", name, "->", pen.strip())
    return d["hands"]["right"], d["hands"]["left"]
exec(open("keys.py").read())

import sys
if len(sys.argv) > 2 and sys.argv[1] == "pose":
    hands = {}
    for name in sys.argv[2].split(","):
        h = eval(name + "()"); side = "right" if name.startswith("R_") else "left"; hands[side] = h
    json.dump({"version": 1, "hands": hands, "camera": CAM}, open(sys.argv[3], "w"), indent=1); sys.exit()
clip = {"version": 1, "fps": 30, "duration": 12, "size": [1440, 1080], "camera": CAM, "keys": KEYS}
json.dump(clip, open("clip.json", "w"), indent=1)
print("keys:", len(KEYS))
