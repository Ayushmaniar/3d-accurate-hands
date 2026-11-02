"""Golden gate for M3: the same pose rendered by fframes (Rust, CPU) and three.js (the
preview) must agree. Compares silhouettes (alpha masks) by intersection over union.

Usage: python tools/golden/compare.py [--out DIR] POSE.json...
Needs release builds of `handkit` and the `handkit-fframes` render example, and the FFmpeg
DLLs on PATH for the latter (see .cargo/config.toml).
"""
import argparse
import os
import subprocess
import sys
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
EXE = ".exe" if os.name == "nt" else ""
HANDKIT = ROOT / "target" / "release" / f"handkit{EXE}"
FFRAMES = ROOT / "target" / "release" / "examples" / f"render{EXE}"
SIZE = 384
GATE_IOU = 0.97


def mask(path):
    a = np.asarray(Image.open(path).convert("RGBA"))
    return a[..., 3] > 127, a[..., :3].astype(float)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=str(ROOT / "bench" / "golden"))
    ap.add_argument("poses", nargs="+")
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    worst = 1.0
    for pose in args.poses:
        name = Path(pose).stem
        a, b = out / f"{name}.fframes.png", out / f"{name}.three.png"
        subprocess.run([str(FFRAMES), pose, str(a), str(SIZE)], check=True, capture_output=True)
        env = dict(os.environ, HANDKIT_GOLDEN="1")
        subprocess.run([str(HANDKIT), "render", pose, "--mesh", "--views", "hero", "--size", str(SIZE), "-o", str(b)], check=True, capture_output=True, env=env)
        (ma, ca), (mb, cb) = mask(a), mask(b)
        iou = (ma & mb).sum() / max((ma | mb).sum(), 1)
        both = ma & mb
        color = np.abs(ca[both].mean(0) - cb[both].mean(0)).max() if both.any() else 0.0
        diff = np.zeros(ma.shape + (3,), np.uint8)
        diff[ma & ~mb] = (255, 80, 80)
        diff[mb & ~ma] = (80, 160, 255)
        diff[both] = (200, 200, 200)
        Image.fromarray(np.concatenate([np.asarray(Image.open(a).convert("RGB")), np.asarray(Image.open(b).convert("RGB")), diff], 1)).save(out / f"{name}.compare.png")
        worst = min(worst, iou)
        print(f"{name:16s} silhouette IoU {iou:.4f}  mean color difference {color:5.1f}/255  {'ok' if iou >= GATE_IOU else 'FAIL'}")
    print(f"worst IoU {worst:.4f} (gate {GATE_IOU})")
    sys.exit(0 if worst >= GATE_IOU else 1)


if __name__ == "__main__":
    main()
