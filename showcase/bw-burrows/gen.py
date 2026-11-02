"""Writes clip.json. Positions are written in the reference's 4:3 frame and mapped to the
player's 16:9 frame (the 4:3 is its centre crop)."""
import json

def sx(x): return round(0.125 + 0.75 * x, 4)

D = 1.40  # default wrist depth (m)

def H(pos, elbow, palm, fingers, shape=None, depth=D, extra=None, **timing):
    h = {"place": {"screen": [sx(pos[0]), pos[1]], "depth": depth,
                   "palm_facing": palm, "fingers_toward": fingers,
                   "elbow_screen": [sx(elbow[0]), elbow[1]]}}
    if shape: h["from"] = shape
    if extra: h.update(extra)
    h.update(timing)
    return h

# shapes
flat = {"index": {"mcp_flex": 8, "pip_flex": 10}, "middle": {"mcp_flex": 8, "pip_flex": 10},
        "ring": {"mcp_flex": 10, "pip_flex": 12}, "pinky": {"mcp_flex": 12, "pip_flex": 14}}
spread = {"index": {"mcp_abd": 12, "mcp_flex": 5, "pip_flex": 8},
          "middle": {"mcp_abd": 2, "mcp_flex": 5, "pip_flex": 8},
          "ring": {"mcp_abd": -8, "mcp_flex": 8, "pip_flex": 10},
          "pinky": {"mcp_abd": -16, "mcp_flex": 10, "pip_flex": 12}}
loose = {"index": {"mcp_flex": 25, "pip_flex": 35}, "middle": {"mcp_flex": 30, "pip_flex": 40},
         "ring": {"mcp_flex": 35, "pip_flex": 45}, "pinky": {"mcp_flex": 40, "pip_flex": 50}}
curl = {"index": {"mcp_flex": 45, "pip_flex": 60}, "middle": {"mcp_flex": 50, "pip_flex": 65},
        "ring": {"mcp_flex": 55, "pip_flex": 65}, "pinky": {"mcp_flex": 60, "pip_flex": 70}}
wrap = {"index": {"mcp_flex": 30, "pip_flex": 30}, "middle": {"mcp_flex": 32, "pip_flex": 32},
        "ring": {"mcp_flex": 35, "pip_flex": 35}, "pinky": {"mcp_flex": 38, "pip_flex": 38}}

wrapL = {"index": {"mcp_flex": 15, "pip_flex": 20}, "middle": {"mcp_flex": 17, "pip_flex": 22},
         "ring": {"mcp_flex": 20, "pip_flex": 25}, "pinky": {"mcp_flex": 24, "pip_flex": 28}}
DOWN = [0, -1, 0]
keys = []
def K(t, right=None, left=None, **kw):
    k = {"t": t, "hands": {}}
    if right: k["hands"]["right"] = right
    if left: k["hands"]["left"] = left
    k.update(kw)
    keys.append(k)

# person's right hand = screen left, left hand = screen right
# 0.0 arms hanging, hands at the hips
K(0.0,
  H((0.20, 0.62), (0.15, 0.22), [0.7, 0, 0.6], [0.15, -1, 0], "relaxed"),
  H((0.83, 0.50), (0.88, 0.15), [-0.4, 0.2, 0.9], [-0.3, -1, 0.2], "relaxed", extra=curl))
K(0.55, left=H((0.82, 0.52), (0.88, 0.16), [-0.4, 0.3, 0.9], [-0.2, -1, 0.2], extra=loose))
# 1.0 right hand presses flat on the apron, left curls inward
K(1.0,
  H((0.27, 0.58), (0.15, 0.20), [0.1, 0, -1], [0.15, -1, 0], extra=spread, move=0.45),
  H((0.85, 0.60), (0.90, 0.22), [-0.9, 0, 0.3], [-0.5, -1, 0], extra=curl, move=0.4))
# 1.5 right swings out, left forearm sweeps across
K(1.5,
  H((0.12, 0.60), (0.14, 0.22), [0.3, 0, 1], [-0.1, -1, 0], "relaxed", move=0.4),
  H((0.45, 0.55), (0.85, 0.22), [-0.6, 0, 0.7], [-0.4, -1, 0], extra=loose, move=0.45))
# 2.0-2.6 hands meet loosely in front of the thighs
K(2.05,
  H((0.255, 0.47), (0.15, 0.17), [0.9, 0, 0.3], [0.08, -1, 0], extra=wrap, depth=1.36, move=0.5),
  H((0.455, 0.47), (0.82, 0.17), [-0.9, 0, 0.3], [-0.1, -1, 0], extra=wrap, depth=1.41, move=0.5))
K(2.6,
  H((0.265, 0.49), (0.15, 0.18), [0.9, 0, 0.3], [0.08, -1, 0], extra=loose, depth=1.36),
  H((0.455, 0.48), (0.82, 0.18), [-0.9, 0, 0.3], [-0.1, -1, 0], extra=loose, depth=1.41))
# 3.0 hands part, both fingers down on the apron
K(3.0,
  H((0.28, 0.55), (0.17, 0.18), [0.2, 0, -1], [0.2, -1, 0], extra=flat, depth=1.44, move=0.35),
  H((0.50, 0.52), (0.82, 0.20), [-0.9, 0, -0.2], [-0.15, -1, 0], extra=flat, depth=1.36, move=0.35))
# 3.5 left hand comes back over the right
K(3.5,
  right=H((0.31, 0.54), (0.17, 0.18), [0.2, 0, -1], [0.2, -1, 0], extra=flat, depth=1.44),
  left=H((0.40, 0.50), (0.80, 0.18), [-0.2, 0, -1], [-0.35, -1, 0], extra=flat, depth=1.36))
# 4.0 right hand flat on the apron, left hand rubbing across
K(4.0,
  H((0.30, 0.62), (0.18, 0.22), [0.1, 0, -1], [0.1, -1, 0], extra=spread, depth=1.44, move=0.4),
  H((0.52, 0.55), (0.80, 0.22), [-0.3, 0, -1], [-0.3, -1, 0], extra=flat, depth=1.35, move=0.4))
# 4.5 right hand rises to the belly, left drops to the side
K(4.6,
  H((0.38, 0.36), (-0.12, 0.30), [0, 1, 0.2], [1, 0.1, 0.3], extra=loose, depth=1.35, move=0.55),
  H((0.75, 0.62), (0.85, 0.25), [-0.5, 0, 0.8], [-0.2, -1, 0], "relaxed", move=0.5))
K(5.1, right=H((0.42, 0.34), (-0.12, 0.29), [0, 1, 0.3], [1, 0, 0.3], extra=curl, depth=1.35))
# 5.5 left hangs low, fingers dangling
K(5.5, left=H((0.66, 0.70), (0.80, 0.33), [-0.2, 0, 1], [-0.1, -1, 0.1], extra=curl, move=0.4))
# 6.0 left rises toward the belly
K(6.0, left=H((0.66, 0.52), (0.86, 0.30), [-0.4, 0.4, 0.8], [-0.6, -1, 0], extra=loose, move=0.4))
# 6.5-8.1 hands rest folded at the belly, left over right
K(6.6,
  H((0.38, 0.36), (-0.12, 0.33), [0, 0, -1], [1, -0.1, 0], extra=wrap, depth=1.445, move=0.5),
  H((0.66, 0.39), (1.12, 0.33), [0, 0, -1], [-1, -0.1, 0], extra=wrapL, depth=1.35, move=0.55))
K(7.4, right=H((0.39, 0.37), (-0.12, 0.33), [0, 0, -1], [1, -0.15, 0], extra=wrap, depth=1.445),
       left=H((0.66, 0.40), (1.12, 0.34), [0, 0, -1], [-1, -0.15, 0], extra=wrapL, depth=1.35))
K(8.1, right=H((0.38, 0.38), (-0.12, 0.33), [0, 0, -1], [1, -0.1, 0], extra=wrap, depth=1.445),
       left=H((0.65, 0.41), (1.12, 0.34), [0, 0, -1], [-1, -0.1, 0], extra=wrapL, depth=1.35))
# 8.5 release: right slides down onto the apron
K(8.55,
  H((0.32, 0.52), (0.14, 0.22), [0.3, 0, -1], [0.1, -1, 0], extra=flat, depth=1.45, move=0.4),
  H((0.56, 0.40), (0.86, 0.30), [0, 0.2, -1], [-1, -0.5, 0], extra=loose, depth=1.38, move=0.4))
# 9.0 right flicks out to the side, palm open; left lands on the apron
K(9.0,
  H((0.14, 0.54), (0.12, 0.20), [0.4, 0.5, 0.8], [0, -1, 0.3], "offering_palm", move=0.3, ease="snappy"),
  H((0.62, 0.55), (0.86, 0.25), [-0.2, 0, -1], [-0.15, -1, 0], extra=flat, move=0.35))
# 9.5 right back to the belly, left hangs at the side
K(9.5,
  H((0.38, 0.36), (-0.12, 0.30), [0, 0.3, -1], [1, -0.2, 0], extra=loose, depth=1.40, move=0.4),
  H((0.72, 0.45), (0.86, 0.15), [-0.6, 0, 0.6], [-0.2, -1, 0], "relaxed", move=0.4))
# 10.0-10.6 left hand dangles, fingers loose
K(10.0, left=H((0.68, 0.57), (0.84, 0.22), [-0.3, 0, 1], [-0.1, -1, 0.1], extra=curl, move=0.4))
K(10.6, left=H((0.68, 0.59), (0.84, 0.23), [-0.3, 0, 1], [-0.15, -1, 0.15], extra=loose))
# 11.0 fold again at the belly
K(11.0,
  H((0.39, 0.38), (-0.12, 0.33), [0, 0, -1], [1, -0.1, 0], extra=wrap, depth=1.445, move=0.45),
  H((0.66, 0.41), (1.12, 0.34), [0, 0, -1], [-1, -0.1, 0], extra=wrapL, depth=1.35, move=0.5))

clip = {"version": 1, "fps": 24, "duration": 12.0,
        "camera": {"position": [0, 0, 1.4], "target": [0, 0, 0], "fov": 24},
        "look": {"style": "realistic", "skin": "medium", "motion_blur": 4},
        "idle": {"amount": 1, "seed": 3},
        "keys": keys}
json.dump(clip, open("clip.json", "w"), indent=1)
print(len(keys), "keys")
