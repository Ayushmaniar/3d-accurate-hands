import json, subprocess, sys, re, os, tempfile
clip = sys.argv[1]; t0, t1 = float(sys.argv[2]), float(sys.argv[3]); site = sys.argv[4] if len(sys.argv) > 4 else "left.middle.knuckle"
H = "../../../target/release/handkit.exe"
tmp = os.path.join(os.environ.get("TMP", "."), "trajpose.json")
prev = None; t = t0
while t <= t1 + 1e-6:
    out = subprocess.run([H, "sample", clip, f"{t:.4f}"], capture_output=True, text=True).stdout
    open(tmp, "w").write(out)
    fk = subprocess.run([H, "fk", tmp], capture_output=True, text=True).stdout
    m = re.search(r"site " + re.escape(site) + r": \[([-\d. ]+)\]", fk)
    w = [float(c) for c in m.group(1).split()]
    if prev:
        v = sum((a - b) ** 2 for a, b in zip(w, prev)) ** 0.5 * 30
        print(f"{t:5.2f} x={w[0]:+.4f} y={w[1]:+.4f} speed={v:.3f} " + "#" * int(v * 200))
    prev = w; t += 1 / 30
