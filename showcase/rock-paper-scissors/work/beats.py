"""Find when each hand's wrist lands (lowest on screen) near each beat, from handkit track."""
import json, subprocess, sys, os
H = os.path.join(os.path.dirname(__file__), "..", "..", "..", "target", "release", "handkit.exe")
clip = sys.argv[1] if len(sys.argv) > 1 else "clip.json"
sites = ["maya.left.wrist_back", "omar.right.wrist_back"]
args = [H, "track", clip, "--fps", "120"]
for s in sites: args += ["--site", s]
out = subprocess.run(args, capture_output=True, text=True).stdout
rows = [json.loads(l.rstrip(',')) for l in out.splitlines() if l.strip().startswith('{')]
if not rows:
    rows = json.loads(out)
for s in sites:
    ts = [r["t"] for r in rows]; ys = [r["sites"][s]["screen"][1] for r in rows]
    lows = []
    for i in range(1, len(ys) - 1):
        if ys[i] >= ys[i-1] and ys[i] > ys[i+1] and ys[i] > 0.5:
            lows.append(ts[i])
    # arrival: first time within 2 px (of 1080) of the low before it
    arr = []
    for lt in lows:
        i = ts.index(lt); y0 = ys[i]
        j = i
        while j > 0 and abs(ys[j-1] - y0) * 1080 < 3: j -= 1
        arr.append((round(ts[j], 3), round(lt, 3)))
    print(s, "lows (within3px, peak):", arr)
