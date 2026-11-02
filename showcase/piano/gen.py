"""Generate clip.json: two hands playing an invisible piano (6 s)."""
import json, sys
ELX, ELY = float(sys.argv[1]), float(sys.argv[2])
CAM = json.loads(sys.argv[3])
OUT = sys.argv[4]

FPS = 30
# hover pose: curved fingers, palm down
BASE = {
    "index":  {"mcp_flex": 22, "pip_flex": 48, "dip_flex": 18},
    "middle": {"mcp_flex": 20, "pip_flex": 50, "dip_flex": 18},
    "ring":   {"mcp_flex": 22, "pip_flex": 50, "dip_flex": 18},
    "pinky":  {"mcp_flex": 25, "pip_flex": 48, "dip_flex": 16},
    "thumb":  {"cmc_abd": 25, "mcp_flex": 10, "ip_flex": 10},
}
PRESS = {  # added on a strike
    "index":  {"mcp_flex": 34, "pip_flex": 6},
    "middle": {"mcp_flex": 34, "pip_flex": 6},
    "ring":   {"mcp_flex": 32, "pip_flex": 6},
    "pinky":  {"mcp_flex": 32, "pip_flex": 6},
    "thumb":  {"cmc_abd": -20, "mcp_flex": 22},
}
LIFT = {f: {k: -v * 0.7 for k, v in d.items()} for f, d in PRESS.items()}
FING = {1: "thumb", 2: "index", 3: "middle", 4: "ring", 5: "pinky"}


def finger(name, extra=None):
    out = dict(BASE[name])
    for k, v in (extra or {}).items():
        out[k] = out.get(k, 0) + v
    return out


def place(side, x, y=0.0, z=0.0):
    s = 1 if side == "right" else -1
    return {
        "wrist": [x, y, z],
        "palm_facing": [0, -1, 0],
        "fingers_toward": [-0.12 * s, -0.08, -1],
        "forearm_toward": [ELX * s, ELY + max(0.0, -y - 0.01) * 8, 1],
    }


keys = {}  # t -> {hand: dict}


def add(t, side, hand):
    t = round(t, 3)
    k = keys.setdefault(t, {})
    k.setdefault(side, {}).update(hand)


def strike(side, t, fingers, x, dip=0.008):
    """fingers lift clearly, strike at t, release; wrist dips with the strike."""
    up = 0.004 if len(fingers) == 1 else 0.014
    pre = {FING[f]: finger(FING[f], LIFT[FING[f]]) for f in fingers}
    hit = {FING[f]: finger(FING[f], PRESS[FING[f]]) for f in fingers}
    rel = {FING[f]: finger(FING[f]) for f in fingers}
    add(t - 0.08, side, {**pre, "move": 0.12, "place": place(side, x, up)})
    add(t, side, {**hit, "move": 0.08, "ease": "snappy", "place": place(side, x, -dip)})
    add(t + 0.17, side, {**rel, "move": 0.14, "place": place(side, x, 0.0)})


# ---- initial: hands enter from above-ish (lowered onto the keys)
RX, LX = 0.11, -0.11
add(0, "right", {f: finger(f) for f in BASE} | {"place": place("right", RX, 0.06, 0.02)})
add(0, "left", {f: finger(f) for f in BASE} | {"place": place("left", LX, 0.06, 0.02)})
add(0.45, "right", {"place": place("right", RX, 0.0), "move": 0.4})
add(0.45, "left", {"place": place("left", LX, 0.0), "move": 0.4})

# ---- music, 8th notes = 0.22 s
E = 0.22
t0 = 0.75
# right hand: rising arpeggio with a hand shift, then a run down
rh = [(1, RX), (2, RX), (3, RX), (5, RX), (3, RX), (2, RX),
      (1, RX + 0.03), (3, RX + 0.03), (5, RX + 0.03), (4, RX + 0.03), (3, RX + 0.03), (2, RX + 0.03),
      (5, RX + 0.05), (4, RX + 0.05), (3, RX + 0.05), (2, RX + 0.05), (1, RX + 0.05), (3, RX + 0.02)]
for i, (f, x) in enumerate(rh):
    strike("right", t0 + i * E, [f], x)
# left hand: chords on beats
lh = [(0, [5, 3, 1], LX), (2, [5, 2], LX), (4, [5, 3, 1], LX), (6, [4, 2, 1], LX - 0.02),
      (8, [5, 3, 1], LX - 0.02), (10, [5, 2, 1], LX - 0.02), (12, [5, 3, 1], LX + 0.01),
      (14, [4, 2], LX + 0.01), (16, [5, 3, 1], LX)]
for i, fs, x in lh:
    strike("left", t0 + i * E, fs, x, dip=0.022)

# ---- final chord both hands, big dip, hold, then lift off
tf = t0 + 18 * E + 0.15  # ~4.86
for side, x, fs in (("right", RX + 0.02, [1, 3, 5]), ("left", LX, [5, 3, 1])):
    pre = {FING[f]: finger(FING[f], {k: v * 1.4 for k, v in LIFT[FING[f]].items()}) for f in fs}
    hit = {FING[f]: finger(FING[f], PRESS[FING[f]]) for f in fs}
    add(tf - 0.22, side, {**pre, "move": 0.18, "pass": True, "place": place(side, x, 0.02, 0.008)})
    add(tf, side, {**hit, "move": 0.1, "ease": "snappy", "place": place(side, x, -0.028)})
    add(tf + 0.5, side, {"move": 0.4, "place": place(side, x, -0.024)})
    add(tf + 1.0, side, {**{f: finger(f) for f in BASE}, "move": 0.4,
                         "place": place(side, x, 0.05, 0.02)})

clip = {
    "version": 1, "fps": FPS, "duration": 6.0, "size": [1920, 1080],
    "camera": CAM,
    "look": {"style": "spiderverse", "skin": "brown", "motion_blur": 4},
    "idle": {"amount": 0.6, "seed": 3},
    "keys": [{"t": t, "hands": h} for t, h in sorted(keys.items())],
}
with open(OUT, "w") as f:
    json.dump(clip, f, indent=1)
print(len(clip["keys"]), "keys")
