import json, math, sys, os
HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, '..', 'clip.json')

def norm(v):
    l = math.sqrt(sum(c*c for c in v)); return [c/l for c in v]
def add(a, b, s=1.0): return [x + s*y for x, y in zip(a, b)]
def r3(v): return [round(c, 4) for c in v]

GAP = 0.029          # vertical spacing between stacked hands (palm surface on back of hand)
SLEEVE = "#1f3a6e"   # one team jersey
# entry order: name, skin, azimuth of the elbow side (deg, x=cos, z=sin; +z toward camera)
PEOPLE = [
    ("ama",  "dark",   126),
    ("ben",  "light",  342),
    ("chen", "tan",    198),
    ("dev",  "brown",   54),
    ("eli",  "medium", 270),
]
ARRIVE = [0.9, 1.75, 2.6, 3.45, 4.3]
OFFS = [(0.004, -0.006), (-0.006, 0.003), (0.005, 0.005), (-0.003, -0.004), (0.0, 0.002)]

def e_of(a):
    a = math.radians(a); return [math.cos(a), 0.0, math.sin(a)]

def flat_place(k, dy=0.0, extra=None):
    name, skin, az = PEOPLE[k]
    e = e_of(az); d = [-c for c in e]
    ox, oz = OFFS[k]
    # palm centre ~0.055 m from the wrist along the fingers: put it over the centre
    w = [e[0]*0.06 + ox, GAP*k + dy, e[2]*0.06 + oz]
    if extra: w = add(w, extra)
    fa = norm([e[0], -0.28 + 0.05*k, e[2]])
    return {"wrist": r3(w), "palm_facing": [0, -1, 0], "fingers_toward": r3(norm([d[0], -0.08, d[2]])),
            "forearm_toward": r3(fa)}

def flat_shape():
    return {"from": "open_palm",
            "index": {"mcp_flex": 8, "pip_flex": 6}, "middle": {"mcp_flex": 8, "pip_flex": 6},
            "ring": {"mcp_flex": 9, "pip_flex": 8}, "pinky": {"mcp_flex": 10, "pip_flex": 10},
            "thumb": {"cmc_abd": 5}}

keys = []
first = {"t": 0, "hands": {}}
for k, (name, skin, az) in enumerate(PEOPLE):
    e = e_of(az)
    start = [e[0]*0.95, 0.12 + (0.25 if e[2] < -0.5 else -0.05), e[2]*0.62 - (0.35 if e[2] < -0.5 else 0)]
    if e[2] > 0.5: start = [e[0]*0.5, -0.35, e[2]*0.6]   # front arms come up from below the frame
    first["hands"][f"{name}.right"] = dict(flat_shape(), place={
        "wrist": r3(start), "palm_facing": r3(norm([0, -1, 0])),
        "fingers_toward": r3(norm([-e[0], 0.2, -e[2]])), "forearm_toward": r3(norm([e[0], -0.3, e[2]]))})
keys.append(first)

for k, (name, skin, az) in enumerate(PEOPLE):
    e = e_of(az); t = ARRIVE[k]
    # waypoint above and behind the stack, then settle down onto it
    wp = flat_place(k, extra=[e[0]*0.05, 0.06, e[2]*0.05])
    keys.append({"t": round(t - 0.22, 3), "pass": True, "hands": {f"{name}.right": {"move": 0.55, "place": wp}}})
    keys.append({"t": t, "hands": {f"{name}.right": {"move": 0.22, "ease": "smooth", "place": flat_place(k)}}})

# the whole stack: a little settle as the last one lands, a dip (ready...) then the lift (break!)
T_DIP = 5.35; T_UP = 6.05
keys.append({"t": T_DIP, "move": 0.4, "ease": "smooth",
             "hands": {f"{n}.right": {"place": flat_place(k, dy=-0.03)} for k, (n, _, _) in enumerate(PEOPLE)}})
lift = {"t": T_UP + 0.42, "hands": {}}
for k, (name, skin, az) in enumerate(PEOPLE):
    e = e_of(az)
    order = len(PEOPLE) - 1 - k          # the top hand leaves first
    w = [e[0]*0.21, 0.12 + 0.012*k, e[2]*0.14]
    lift["hands"][f"{name}.right"] = {
        "from": "spread_palm", "move": 0.42 - 0.02*order, "ease": "snappy", "anticipate": 0.0,
        "place": {"wrist": r3(w), "palm_facing": r3(norm([-e[0], 0.15, -e[2] + 0.4])),
                  "fingers_toward": r3(norm([e[0]*0.35, 1, e[2]*0.25])),
                  "forearm_toward": r3(norm([e[0]*0.6, -1, e[2]*0.6]))}}
keys.append(lift)

clip = {"version": 1, "name": "team_huddle",
        "description": "Five teammates put their right hands in one after another, stacked, then all lift together.",
        "fps": 30, "duration": 8.0, "size": [1920, 1080],
        "camera": {"position": [0, 0.42, 0.78], "target": [0, 0.07, 0], "fov": 38},
        "look": {"style": "realistic", "motion_blur": 4},
        "idle": {"amount": 0.4},
        "people": {n: {"look": {"skin": s, "arm": {"sleeve": SLEEVE, "length": 0.1}}} for n, s, _ in PEOPLE},
        "keys": sorted(keys, key=lambda k: k["t"])}
json.dump(clip, open(OUT, "w"), indent=1)
print("wrote", OUT)
