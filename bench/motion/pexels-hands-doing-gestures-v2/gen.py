"""Generates clip.json from a table of hand keys read off the reference (480x270 frame pixels)."""
import json, math, sys

FRAME_H = 0.31          # world meters spanned by the frame height at z = 0
S = FRAME_H / 270.0
CAM_Z = 0.5

def px(p):
    return [round((p[0] - 240) * S, 4), round((135 - p[1]) * S, 4)]

def norm(v):
    l = math.sqrt(sum(c * c for c in v))
    return [round(c / l, 3) for c in v]

# finger shapes (joint values); thumb included
SHAPES = {
    # graceful upright hand: index long, others curving a little more toward the pinky
    "ballet": {"palm_cup": 6,
               "thumb": {"cmc_abd": 18, "cmc_flex": 25, "cmc_rot": 20, "mcp_flex": 5, "ip_flex": 10},
               "index": {"mcp_flex": 5, "pip_flex": 8, "mcp_abd": 2},
               "middle": {"mcp_flex": 15, "pip_flex": 18},
               "ring": {"mcp_flex": 22, "pip_flex": 22, "mcp_abd": 3},
               "pinky": {"mcp_flex": 28, "pip_flex": 22, "mcp_abd": 6}},
    # upright hand with the middle, ring and pinky curled forward, thumb out (end section)
    "curl": {"palm_cup": 8,
             "thumb": {"cmc_abd": 18, "cmc_flex": -5, "cmc_rot": 10, "mcp_flex": 5, "ip_flex": 10},
             "index": {"mcp_flex": 8, "pip_flex": 25, "mcp_abd": 0},
             "middle": {"mcp_flex": 18, "pip_flex": 55, "mcp_abd": -6},
             "ring": {"mcp_flex": 22, "pip_flex": 60, "mcp_abd": 4},
             "pinky": {"mcp_flex": 25, "pip_flex": 55, "mcp_abd": 8}},
    # open fan, fingers spread and nearly straight, thumb along the index
    "fan": {"palm_cup": 4,
            "thumb": {"cmc_abd": 12, "cmc_flex": 22, "cmc_rot": 15, "mcp_flex": 5, "ip_flex": 10},
            "index": {"mcp_flex": 5, "pip_flex": 8, "mcp_abd": 10},
            "middle": {"mcp_flex": 10, "pip_flex": 12, "mcp_abd": 0},
            "ring": {"mcp_flex": 15, "pip_flex": 15, "mcp_abd": 8},
            "pinky": {"mcp_flex": 20, "pip_flex": 15, "mcp_abd": 16}},
    # palm down, fingers drooping from the knuckles, index the straightest
    "droop": {"palm_cup": 5,
              "thumb": {"cmc_abd": 20, "cmc_flex": 25, "cmc_rot": 15, "mcp_flex": 10, "ip_flex": 15},
              "index": {"mcp_flex": 5, "pip_flex": 8, "mcp_abd": 3},
              "middle": {"mcp_flex": 15, "pip_flex": 15},
              "ring": {"mcp_flex": 25, "pip_flex": 18, "mcp_abd": 3},
              "pinky": {"mcp_flex": 30, "pip_flex": 18, "mcp_abd": 6}},
    # swan-neck hand: strong knuckle bend, fingers fairly straight beyond it
    "hook": {"palm_cup": 6,
             "thumb": {"cmc_abd": 20, "cmc_flex": 25, "cmc_rot": 15, "mcp_flex": 10, "ip_flex": 15},
             "index": {"mcp_flex": 40, "pip_flex": 10, "mcp_abd": 2},
             "middle": {"mcp_flex": 50, "pip_flex": 12},
             "ring": {"mcp_flex": 55, "pip_flex": 15, "mcp_abd": 3},
             "pinky": {"mcp_flex": 55, "pip_flex": 18, "mcp_abd": 6}},
    # strong swan neck: knuckles bent hard, fingers hanging
    "swan": {"palm_cup": 6,
             "thumb": {"cmc_abd": 20, "cmc_flex": 25, "cmc_rot": 15, "mcp_flex": 10, "ip_flex": 15},
             "index": {"mcp_flex": 75, "pip_flex": 22, "mcp_abd": 2},
             "middle": {"mcp_flex": 82, "pip_flex": 26},
             "ring": {"mcp_flex": 85, "pip_flex": 28, "mcp_abd": 3},
             "pinky": {"mcp_flex": 85, "pip_flex": 30, "mcp_abd": 6}},
    # light hook: fingers bent at the knuckles, mostly straight
    "hooklite": {"palm_cup": 6,
             "thumb": {"cmc_abd": 15, "cmc_flex": 30, "cmc_rot": 20, "mcp_flex": 5, "ip_flex": 10},
             "index": {"mcp_flex": 25, "pip_flex": 10, "mcp_abd": 4},
             "middle": {"mcp_flex": 35, "pip_flex": 15},
             "ring": {"mcp_flex": 40, "pip_flex": 18, "mcp_abd": 4},
             "pinky": {"mcp_flex": 40, "pip_flex": 20, "mcp_abd": 8}},
    # ballet hand: index tall, middle and ring dropping toward the thumb
    "ballerina": {"palm_cup": 12,
             "thumb": {"cmc_abd": 25, "cmc_flex": 22, "cmc_rot": 30, "mcp_flex": 10, "ip_flex": 10},
             "index": {"mcp_flex": 8, "pip_flex": 15, "mcp_abd": 2},
             "middle": {"mcp_flex": 15, "pip_flex": 55, "mcp_roll": 10},
             "ring": {"mcp_flex": 18, "pip_flex": 50, "mcp_roll": 10},
             "pinky": {"mcp_flex": 15, "pip_flex": 25, "mcp_abd": 10}},
    # palm down, index reaching out straight, the other fingers hanging curled
    "reach": {"palm_cup": 8,
             "thumb": {"cmc_abd": 25, "cmc_flex": 25, "cmc_rot": 15, "mcp_flex": 10, "ip_flex": 15},
             "index": {"mcp_flex": 0, "pip_flex": 10, "mcp_abd": 5},
             "middle": {"mcp_flex": 20, "pip_flex": 28},
             "ring": {"mcp_flex": 30, "pip_flex": 32, "mcp_abd": 2},
             "pinky": {"mcp_flex": 35, "pip_flex": 30, "mcp_abd": 6}},
}

def hand_entry(k):
    """k: dict(wrist=(x,y), knuck=(x,y), fz=z of finger dir, palm=[x,y,z], arm=[x,y,z] forearm_toward, z=depth, shape=..., over={...})."""
    w = px(k["wrist"])
    w.append(k.get("z", 0.0))
    kn = k["knuck"]
    d = [kn[0] - k["wrist"][0], -(kn[1] - k["wrist"][1])]
    l = math.hypot(*d)
    ft = norm([d[0] / l, d[1] / l, k.get("fz", 0.0)])
    place = {"wrist": w, "palm_facing": norm(k["palm"]), "fingers_toward": ft}
    if "arm" in k:
        place["forearm_toward"] = norm(k["arm"])
    e = json.loads(json.dumps(SHAPES[k.get("shape", "ballet")]))
    for dig, vals in k.get("over", {}).items():
        if isinstance(vals, dict):
            e.setdefault(dig, {}).update(vals)
        else:
            e[dig] = vals
    e["place"] = place
    return e

def build(keys, duration, out="clip.json"):
    clip = {"version": 1, "fps": 30, "duration": duration, "size": [1920, 1080],
            "camera": {"position": [0, 0, CAM_Z], "target": [0, 0, 0], "fov": 35},
            "idle": {"amount": 0.6, "seed": 3},
            "keys": []}
    for k in keys:
        key = {"t": k["t"]}
        for opt in ("move", "ease", "pass", "spring", "cascade"):
            if opt in k:
                key[opt] = k[opt]
        key["hands"] = {}
        for side in ("left", "right"):
            if side in k:
                key["hands"][side] = hand_entry(k[side])
        clip["keys"].append(key)
    with open(out, "w") as f:
        json.dump(clip, f, indent=1)

if __name__ == "__main__":
    from keys import KEYS, DURATION
    build(KEYS, DURATION, sys.argv[1] if len(sys.argv) > 1 else "clip.json")
