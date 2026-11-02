"""Generate clip.json from a compact keyframe table read off the reference.

Screen coordinates are in the reference's 1920x1080 pixels. The camera is set so the
plane z=0 maps 1:1 onto that frame (M metres per pixel).
For each hand and key:
  w   : wrist (px, py) [, z metres]
  arm : forearm direction, elbow->wrist, as a screen angle in degrees from straight up,
        positive leaning right (+X); optional depth via "armz" (z component, + toward camera)
  fin : direction the (straight) fingers point, world vector
  palm: direction the palm faces, world vector
  then finger values.
The hand frame is placed directly and wrist_flex / wrist_dev are solved so that the
forearm points along `arm`.
"""
import json, math, sys

M = 0.000265  # metres per reference pixel at z = 0
FOV = 30.0
DIST = 1080 * M / (2 * math.tan(math.radians(FOV / 2)))


def w2(px, py, z=0.0):
    return [round((px - 960) * M, 4), round((540 - py) * M, 4), z]


def norm(v):
    n = math.sqrt(sum(c * c for c in v))
    return [c / n for c in v]


def cross(a, b):
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


WARN = []


def wrist_angles(side, fin, palm, arm_deg, armz):
    y = norm(fin)
    z = [p - dot(palm, y) * c for p, c in zip(palm, y)]
    z = norm(z)
    t = cross(y, z)
    x = t if side == "right" else [-c for c in t]  # toward the thumb
    a = math.radians(arm_deg)
    f = norm([math.sin(a), math.cos(a), armz])
    fx, fy, fz = dot(f, x), dot(f, y), dot(f, z)
    dev = -math.degrees(math.asin(max(-1, min(1, fx))))
    flex = math.degrees(math.atan2(-fz, fy))
    return flex, dev


def finger_block(h):
    out = {}
    # fingers: "f": {"index": [mcp, pip(, abd)], ...} or shorthand curl
    for name in ("index", "middle", "ring", "pinky"):
        if name in h:
            v = h[name]
            d = {"mcp_flex": v[0], "pip_flex": v[1]}
            if len(v) > 2:
                d["mcp_abd"] = v[2]
            out[name] = d
    if "thumb" in h:
        out["thumb"] = h["thumb"]
    if "cup" in h:
        out["palm_cup"] = h["cup"]
    return out


def hand(side, h):
    px, py = h["w"][0] * 4, h["w"][1] * 4
    if "hd" in h:
        a = math.radians(h["hd"])
        h["fin"] = (math.sin(a), math.cos(a), h.get("hz", 0.0))
    z = h["w"][2] if len(h["w"]) > 2 else 0.0
    if "armz" in h:
        flex, dev = wrist_angles(side, h["fin"], h["palm"], h["arm"], h["armz"])
    else:
        # choose the forearm's depth tilt so the wrist stays inside its limits
        best = None
        for e in range(h.get("tilt_min", -20), 51, 2):
            armz = math.tan(math.radians(e))
            f, d = wrist_angles(side, h["fin"], h["palm"], h["arm"], armz)
            viol = max(0, -70 - f, f - 80) + max(0, -25 - d, d - 35)
            cost = viol + 0.02 * abs(e) + (0.01 * abs(e) if e < 0 else 0)
            if best is None or cost < best[0]:
                best = (cost, f, d, e)
        _, flex, dev, e = best
        if abs(e) > 20 and "tilt_min" not in h:
            WARN.append(f"{side} t={h.get('_t')}: forearm tilted {e} deg in depth")
    lim_f = (-70, 80)
    lim_d = (-25, 35)
    cf = max(lim_f[0], min(lim_f[1], flex))
    cd = max(lim_d[0], min(lim_d[1], dev))
    if abs(cf - flex) > 0.5 or abs(cd - dev) > 0.5:
        WARN.append(f"{side} t={h.get('_t')}: flex {flex:.0f} dev {dev:.0f} clamped")
    out = {
        "place": {"wrist": w2(px, py, z), "palm_facing": [round(c, 3) for c in norm(h["palm"])],
                  "fingers_toward": [round(c, 3) for c in norm(h["fin"])]},
        "twist": h.get("twist", 0),
        "wrist_flex": round(cf, 1),
        "wrist_dev": round(cd, 1),
    }
    out.update(finger_block(h))
    return out


def build(keys, duration, out="clip.json", move_scale=1.0, ease="smooth"):
    ks = []
    prev_t = 0
    for i, k in enumerate(keys):
        hands = {}
        for side in ("left", "right"):
            if side in k:
                k[side]["_t"] = k["t"]
                hands[side] = hand(side, k[side])
        key = {"t": k["t"], "hands": hands}
        if i > 0:
            gap = k["t"] - prev_t
            mv = k.get("move", gap * move_scale)
            key["move"] = round(mv, 3)
            key["ease"] = k.get("ease", ease)
            # overlapping moves lead the keyed path by (move - gap) / 2: shift to centre them
            key["t"] = round(k["t"] + max(0.0, mv - gap) / 2, 3)
        prev_t = k["t"]
        ks.append(key)
    clip = {
        "version": 1,
        "name": "pexels-hands-doing-gestures seg01",
        "fps": 30,
        "duration": duration,
        "size": [1920, 1080],
        "camera": {"position": [0, 0, round(DIST, 4)], "target": [0, 0, 0], "fov": FOV},
        "idle": {"amount": 0.6, "seed": 3},
        "keys": ks,
    }
    with open(out, "w") as fh:
        json.dump(clip, fh, indent=1)
    for w in WARN:
        print("WARN", w, file=sys.stderr)


if __name__ == "__main__":
    from keys import KEYS, DURATION, MOVE_SCALE, EASE
    build(KEYS, DURATION, move_scale=MOVE_SCALE, ease=EASE)
