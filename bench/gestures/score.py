"""Scores gesture runs with the same measures, old experiment runs included.

Usage: python bench/gestures/score.py [--sheets] RUN_DIR...
Contact error = |surface gap| in mm between the segments that should touch; aim error = angle
between the index line (knuckle to tip) and the T5 target. --sheets also renders each pose
and writes side-by-side sheets with the reference photos to bench/gestures/sheets/.
"""
import json
import os
import subprocess
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
HANDKIT = str(ROOT / "target" / "debug" / ("handkit.exe" if os.name == "nt" else "handkit"))
REFS = {1: "9", 2: "1", 3: "7", 4: "6"}
# Segments that must touch, per task (any segment of the second digit counts for T1).
CONTACTS = {
    1: [("right.thumb.distal", "right.index.*")],
    2: [("left.index.distal", "right.thumb.distal"), ("left.thumb.distal", "right.index.distal")],
    3: [("left.thumb.distal", "right.thumb.distal"), ("left.index.distal", "right.index.distal")],
    6: [("right.thumb.distal", "right.index.distal")],
}
T5_TARGET = np.array([-0.6, 0.3, -0.4])


def seg_dist(p1, q1, p2, q2):
    d1, d2, r = q1 - p1, q2 - p2, p1 - p2
    a, e, f = d1 @ d1, d2 @ d2, d2 @ r
    c, b = d1 @ r, d1 @ d2
    den = a * e - b * b
    s = np.clip((b * f - c * e) / den, 0, 1) if den > 1e-12 else 0.0
    t = (b * s + f) / e
    if t < 0:
        t, s = 0.0, np.clip(-c / a, 0, 1)
    elif t > 1:
        t, s = 1.0, np.clip((b - c) / a, 0, 1)
    return np.linalg.norm((p1 + d1 * s) - (p2 + d2 * t))


def gap(caps, a, b):
    def pick(name):
        return [c for c in caps if c["label"] == name or (name.endswith("*") and c["label"].startswith(name[:-1]) and not c["label"].endswith("metacarpal"))]
    best = None
    for x in pick(a):
        for y in pick(b):
            g = (seg_dist(*map(np.array, (x["a"], x["b"], y["a"], y["b"]))) - x["r"] - y["r"]) * 1000
            best = g if best is None else min(best, g)
    return best


def run(*args):
    return subprocess.run([HANDKIT, *args], capture_output=True, text=True, check=True).stdout


def score(run_dir):
    rows = []
    for t in range(1, 8):
        f = run_dir / f"T{t}.json"
        if not f.exists():
            rows.append((t, None))
            continue
        geo = json.loads(run("geometry", str(f)))
        q = json.loads(run("query", str(f), "--json"))
        caps = geo["capsules"]
        r = {"violations": len(q["joint_limit_violations"]), "penetrations": len(q["penetrations"])}
        if t in CONTACTS:
            r["contact_mm"] = [round(abs(gap(caps, a, b)), 1) for a, b in CONTACTS[t]]
        if t == 5:
            base = next(c for c in caps if c["label"] == "right.index.proximal")["a"]
            tip = next(s for s in geo["sites"] if s["name"] == "right.index.tip")["pos"]
            ray, to = np.array(tip) - base, T5_TARGET - base
            r["aim_deg"] = round(float(np.degrees(np.arccos(ray @ to / np.linalg.norm(ray) / np.linalg.norm(to)))), 1)
        rows.append((t, r))
    return rows


def sheets(run_dirs):
    from PIL import Image, ImageDraw
    out = ROOT / "bench" / "gestures" / "sheets"
    out.mkdir(parents=True, exist_ok=True)
    H = 400
    for t in range(1, 8):
        tiles = []
        ref = REFS.get(t)
        labels = [f"T{t} reference" if ref else f"T{t} (no photo)"] + [d.parent.name + "/" + d.name if d.parent.name == "runs" and False else d.name for d in run_dirs]
        paths = [ROOT / "experiments" / "joint-angles" / "refs" / f"{ref}.png" if ref else None]
        for d in run_dirs:
            png = d / f"T{t}.score.png"
            if (d / f"T{t}.json").exists():
                run("render", str(d / f"T{t}.json"), "-o", str(png), "--views", "front,hero", "--size", "300")
            paths.append(png)
        for p, label in zip(paths, labels):
            img = Image.new("RGB", (H * 2 if p and p != paths[0] else H, H + 24), (24, 26, 30))
            if p and p.exists():
                src = Image.open(p).convert("RGB")
                src.thumbnail((img.width, H))
                img.paste(src, ((img.width - src.width) // 2, 24))
            ImageDraw.Draw(img).text((6, 5), label, fill=(235, 235, 235))
            tiles.append(img)
        sheet = Image.new("RGB", (sum(i.width for i in tiles), H + 24))
        x = 0
        for i in tiles:
            sheet.paste(i, (x, 0))
            x += i.width
        sheet.save(out / f"T{t}.png")
    print("sheets in", out)


if __name__ == "__main__":
    args = [a for a in sys.argv[1:] if a != "--sheets"]
    dirs = [Path(a).resolve() for a in args]
    for d in dirs:
        rows = score(d)
        contacts = [m for _, r in rows if r for m in r.get("contact_mm", [])]
        pens = sum(r["penetrations"] for _, r in rows if r)
        viol = sum(r["violations"] for _, r in rows if r)
        aim = next((r.get("aim_deg") for t, r in rows if t == 5 and r), None)
        print(f"{d.name:16} contacts mean {np.mean(contacts):5.1f} mm, max {max(contacts):5.1f} mm | penetrations {pens:2} | violations {viol} | T5 aim {aim} deg")
        for t, r in rows:
            print(f"    T{t}: {r}")
    if "--sheets" in sys.argv:
        sheets(dirs)
