"""Generate clip.json: two people play rock paper scissors on a 120 BPM grid.

Left player maya (light skin, teal sleeve) uses her left hand from the left edge, right
player omar (dark skin, rust sleeve) his right hand from the right edge, mirror images:
fingers toward each other, palms to the camera, thumbs up.
Round 1: pumps land 0.5, 1.0, 1.5; both throw scissors at 2.0 (tie).
Round 2: pumps land 2.5, 3.0, 3.5; maya throws paper, omar rock at 4.0; react to 5.0.
"""
import json, copy, os

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "clip.json")

# ---- shapes (explicit joint values, no library goals) ----
FIST = {
    "palm_cup": 10,
    "thumb": {"cmc_abd": 44.1, "cmc_flex": 2.8, "cmc_rot": 48.8, "ip_flex": 24.0, "mcp_flex": 48.7},
    "index": {"mcp_abd": 0, "mcp_flex": 85, "pip_flex": 95, "dip_flex": 40},
    "middle": {"mcp_abd": 0, "mcp_flex": 88, "pip_flex": 98, "dip_flex": 40},
    "ring": {"mcp_abd": -3, "mcp_flex": 88, "pip_flex": 98, "dip_flex": 40},
    "pinky": {"mcp_abd": 0, "mcp_flex": 85, "pip_flex": 88, "dip_flex": 40},
}
SCISSORS = {
    "palm_cup": 10,
    "thumb": {"cmc_abd": 42.7, "cmc_flex": 15.8, "cmc_rot": 48.8, "ip_flex": 26.1, "mcp_flex": 47.2},
    "index": {"mcp_abd": 16, "mcp_flex": 0, "pip_flex": 3, "dip_flex": 2},
    "middle": {"mcp_abd": -14, "mcp_flex": 0, "pip_flex": 3, "dip_flex": 2},
    "ring": {"mcp_abd": -3, "mcp_flex": 88, "pip_flex": 98, "dip_flex": 40},
    "pinky": {"mcp_abd": 0, "mcp_flex": 85, "pip_flex": 88, "dip_flex": 40},
}
PAPER = {
    "palm_cup": 0,
    "thumb": {"cmc_abd": 8, "cmc_flex": 4, "cmc_rot": 5, "ip_flex": 4, "mcp_flex": 4},
    "index": {"mcp_abd": -4, "mcp_flex": 0, "pip_flex": 2, "dip_flex": 2},
    "middle": {"mcp_abd": -2, "mcp_flex": 0, "pip_flex": 2, "dip_flex": 2},
    "ring": {"mcp_abd": -1, "mcp_flex": 0, "pip_flex": 3, "dip_flex": 2},
    "pinky": {"mcp_abd": -4, "mcp_flex": 1, "pip_flex": 4, "dip_flex": 3},
}

# ---- framing ----
DEPTH = 0.86
X = {"maya": 0.215, "omar": 0.785}          # wrist x on screen
SIDE = {"maya": 1, "omar": -1}            # fingers point +x for maya, -x for omar
ELBOW = {"maya": [-0.06, 0.9], "omar": [1.06, 0.9]}
Y_TOP, Y_MID, Y_BOT = 0.37, 0.46, 0.60
HAND = {"maya": "maya.left", "omar": "omar.right"}


def place(who, y, tilt=0.0, dx=0.0):
    """Wrist at (x, y) on screen; fingers toward the opponent, tilted up by `tilt` (y comp)."""
    s = SIDE[who]
    return {
        "screen": [X[who] + s * dx, y],
        "depth": DEPTH,
        "palm_facing": [0, 0.12, 1],
        "fingers_toward": [s * 1.0, tilt, 0],
        "elbow_screen": ELBOW[who],
    }


def hand(who, y, shape=None, tilt=0.0, dx=0.0, **timing):
    h = {}
    if shape is not None:
        h.update(copy.deepcopy(shape))
    h["place"] = place(who, y, tilt, dx)
    h.update(timing)
    return h


keys = []


def key(t, maya, omar, **timing):
    k = {"t": round(t, 3), "hands": {HAND["maya"]: maya, HAND["omar"]: omar}}
    k.update(timing)
    keys.append(k)


def both(t, y, shape=None, tilt=0.0, **timing):
    key(t, hand("maya", y, shape, tilt), hand("omar", y, shape, tilt), **timing)


UP_TILT, BOT_TILT = 0.30, -0.05   # wrist cocked up at the top of a pump, level at the bottom
DOWN = 0.20                        # down stroke length (s)

# t=0: fists in frame, part way up, rising
both(0.0, Y_MID, FIST, tilt=0.12)


# round 1 pumps
for b in (0.5, 1.0, 1.5):
    both(b - DOWN - 0.03, Y_TOP, FIST, tilt=UP_TILT, ease="smooth")
    both(b, Y_BOT, FIST, tilt=BOT_TILT, move=DOWN, ease="snappy")


def throw(t_land, maya_shape, omar_shape, maya_tilt, omar_tilt):
    # top, then a flowing waypoint half way down still in a fist, then the fast throw
    both(t_land - DOWN - 0.03, Y_TOP, FIST, tilt=UP_TILT, ease="smooth")
    both(t_land - 0.12, (Y_TOP + Y_BOT) / 2 + 0.02, FIST, tilt=0.1, move=DOWN - 0.12, pass_=True)
    key(t_land,
        hand("maya", Y_BOT, maya_shape, tilt=maya_tilt, move=0.12, ease="snappy", cascade=0.008),
        hand("omar", Y_BOT, omar_shape, tilt=omar_tilt, move=0.12, ease="snappy", cascade=0.008))


throw(2.0, SCISSORS, SCISSORS, 0.42, 0.42)

# hold the tie, then back to fists while rising for round 2
both(2.14, Y_BOT, SCISSORS, tilt=0.42)
for b in (2.5, 3.0, 3.5):
    both(b - DOWN - 0.03, Y_TOP, FIST, tilt=UP_TILT, ease="smooth")
    both(b, Y_BOT, FIST, tilt=BOT_TILT, move=DOWN, ease="snappy")

throw(4.0, PAPER, FIST, 0.32, BOT_TILT)

# reaction: maya's paper gives a quick small flick up and back; omar's rock sinks
flick = copy.deepcopy(PAPER)
key(4.38, hand("maya", Y_BOT - 0.025, flick, tilt=0.55, move=0.1, ease="snappy"),
    hand("omar", Y_BOT, FIST, tilt=BOT_TILT, move=0.2))
key(4.56, hand("maya", Y_BOT, PAPER, tilt=0.32, move=0.16, ease="natural"),
    hand("omar", Y_BOT, FIST, tilt=BOT_TILT))
key(4.95, hand("maya", Y_BOT, PAPER, tilt=0.32),
    hand("omar", Y_BOT + 0.045, FIST, tilt=-0.25, move=0.6, ease="smooth"))


def clean(o):
    """pass_ -> pass, drop None."""
    if isinstance(o, dict):
        r = {}
        for k, v in o.items():
            if v is None:
                continue
            r["pass" if k == "pass_" else k] = clean(v)
        return r
    if isinstance(o, list):
        return [clean(v) for v in o]
    return o


clip = {
    "version": 1,
    "name": "rock_paper_scissors",
    "description": "Two players, two rounds of rock paper scissors on a 120 BPM grid: a scissors tie, then paper beats rock.",
    "fps": 30,
    "duration": 5.0,
    "size": [1920, 1080],
    "camera": {"position": [0, 0.15, 0.85], "target": [0, 0.02, 0], "fov": 25},
    "look": {"style": "realistic", "motion_blur": 4},
    "idle": {"amount": 0.4},
    "people": {
        "maya": {"look": {"skin": "light", "arm": {"sleeve": "#2a7f86", "length": 0.16}}},
        "omar": {"look": {"skin": "dark", "arm": {"sleeve": "#c0562f", "length": 0.16}}},
    },
    "keys": clean(keys),
}
with open(OUT, "w") as f:
    json.dump(clip, f, indent=1)
print("wrote", os.path.normpath(OUT), len(keys), "keys")
