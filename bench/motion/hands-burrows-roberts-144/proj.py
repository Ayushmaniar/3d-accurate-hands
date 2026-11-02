"""Map between 4:3 frame fractions and world points for the clip camera.
python proj.py fx fy depth_point_z  -> world point on plane z=depth
python proj.py w x y z             -> frame fraction"""
import sys, json, math
import numpy as np
cam = json.load(open("clip.json"))["camera"] if len(sys.argv) > 1 and sys.argv[1] != "x" else None
P = np.array(cam["position"], float); T = np.array(cam["target"], float); fov = cam["fov"]
f = T - P; f /= np.linalg.norm(f)
r = np.cross(f, [0, 1, 0]); r /= np.linalg.norm(r)
u = np.cross(r, f)
th = math.tan(math.radians(fov) / 2); tw = th * 4 / 3
def to_frac(X):
    d = np.array(X, float) - P
    z = d @ f
    return (0.5 + (d @ r) / z / tw / 2, 0.5 - (d @ u) / z / th / 2)
def from_frac(fx, fy, zp):
    ray = f + (2 * fx - 1) * tw * r - (2 * fy - 1) * th * u
    s = (zp - P[2]) / ray[2]
    return P + s * ray
a = sys.argv[1:]
if a[0] == "w":
    print("%.3f %.3f" % to_frac([float(v) for v in a[1:4]]))
else:
    print(np.round(from_frac(float(a[0]), float(a[1]), float(a[2])), 3).tolist())
