"""Samples the clip every frame and writes src/tip.json: the right index fingertip as
[screen_x_px, screen_y_px, distance_m_from_camera] per frame (1920x1080, the clip's camera:
on the z axis looking down -z, vertical fov). Run after editing clip.json, then rebuild."""
import json, math, subprocess, sys, os

here = os.path.dirname(os.path.abspath(__file__))
H = os.path.join(here, "..", "..", "target", "release", "handkit")
clip = json.load(open(os.path.join(here, "clip.json")))
cam = clip["camera"]
cz, fov = cam["position"][2], cam["fov"]
W, Hh = 1920, 1080
k = math.tan(math.radians(fov) / 2)
n = int(round(clip["duration"] * clip["fps"]))
out = []
tmp = os.path.join(here, "_pose.json")
for f in range(n):
    t = f / clip["fps"]
    pose = subprocess.run([H, "sample", "clip.json", f"{t:.4f}"], cwd=here, capture_output=True, text=True, check=True).stdout
    open(tmp, "w").write(pose)
    q = json.loads(subprocess.run([H, "query", "--json", tmp], cwd=here, capture_output=True, text=True, check=True).stdout)
    x, y, z = q["fingertips_m"]["right.index.tip"]
    d = cz - z
    sx = W / 2 + x / (d * k) * (Hh / 2)
    sy = Hh / 2 - y / (d * k) * (Hh / 2)
    out.append([round(sx, 1), round(sy, 1), round(d, 4)])
os.remove(tmp)
json.dump(out, open(os.path.join(here, "src", "tip.json"), "w"))
for f in range(0, n, 6):
    print(f / clip["fps"], out[f])
