"""Compares the fframes and three.js renders of each look: mean colour difference inside the
hand, alpha IoU, and a crop sheet (fframes left, three.js right) for a close look.

  python tools/looks/diff.py <dir with ff_<style>.png and three_<style>.png> [x0 y0 x1 y1]
"""
import sys
from pathlib import Path

import numpy as np
from PIL import Image

STYLES = ["realistic", "toon", "spiderverse", "arcane", "clay"]
BACKDROP = {"realistic": (0x76, 0x78, 0x7B), "toon": (0xF3, 0xEE, 0xE2), "spiderverse": (0xFB, 0xF8, 0xF0), "arcane": (0x16, 0x1A, 0x24), "clay": (0xBD, 0xB8, 0xB2)}


def main():
    d = Path(sys.argv[1])
    box = tuple(int(v) for v in sys.argv[2:6]) if len(sys.argv) >= 6 else (300, 120, 780, 600)
    crops = []
    for s in STYLES:
        a = np.asarray(Image.open(d / f"ff_{s}.png").convert("RGB")).astype(np.float32)
        b = np.asarray(Image.open(d / f"three_{s}.png").convert("RGB")).astype(np.float32)
        bg = np.array(BACKDROP[s], np.float32)
        ma = np.abs(a - bg).sum(-1) > 12
        mb = np.abs(b - bg).sum(-1) > 12
        iou = (ma & mb).sum() / max((ma | mb).sum(), 1)
        both = ma & mb
        diff = np.abs(a - b)[both].mean(0)
        mean_a, mean_b = a[both].mean(0), b[both].mean(0)
        lin = lambda c: np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)
        ratio = np.median(lin(a[both] / 255) / np.maximum(lin(b[both] / 255), 1e-3), axis=0)
        print(f"{s:12s} IoU {iou:.3f}  mean |diff| RGB {diff.round(1)}  linear ff/three {ratio.round(3)}")
        crops.append(np.concatenate([a[box[1]:box[3], box[0]:box[2]], b[box[1]:box[3], box[0]:box[2]]], 1))
    Image.fromarray(np.concatenate(crops, 0).astype(np.uint8)).save(d / "crops.png")


if __name__ == "__main__":
    main()
