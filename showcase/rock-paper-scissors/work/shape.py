"""When does each throw's finger shape finish? Index-tip-to-wrist distance, 120 fps."""
import json, subprocess, sys, os, math
H = os.path.join(os.path.dirname(__file__), "..", "..", "..", "target", "release", "handkit.exe")
clip = sys.argv[1]
hands = ["maya.left", "omar.right"]
args = [H, "track", clip, "--fps", "120"]
for h in hands: args += ["--site", h + ".wrist_back", "--site", h + ".index.tip", "--site", h + ".pinky.tip"]
out = subprocess.run(args, capture_output=True, text=True).stdout
rows = [json.loads(l.rstrip(',')) for l in out.splitlines() if l.strip().startswith('{')]
for h in hands:
    for f in ("index", "pinky"):
        d = [(r["t"], math.dist(r["sites"][h + ".wrist_back"]["world"], r["sites"][h + "." + f + ".tip"]["world"])) for r in rows]
        for beat in (2.0, 4.0):
            win = [x for x in d if beat - 0.3 <= x[0] <= beat + 0.3]
            final = [x for x in d if abs(x[0] - (beat + 0.1)) < 0.005][0][1]
            start = [x for x in d if abs(x[0] - (beat - 0.25)) < 0.005][0][1]
            if abs(final - start) < 0.01:
                continue
            go = next(t for t, v in win if abs(v - start) > 0.1 * abs(final - start))
            done = next(t for t, v in win if abs(v - final) < 0.1 * abs(final - start))
            print(f"{h} {f} beat {beat}: shape moves {go:.3f} -> 90% at {done:.3f}  ({start*100:.1f} -> {final*100:.1f} cm)")
