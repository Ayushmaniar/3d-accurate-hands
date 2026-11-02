"""Side-by-side sheets: reference photo next to each run's render, one sheet per task.

Usage: python compare.py run1 run2 ...   (folders under runs/, each with T<n>.png)
"""
import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).parent
REFS = {1: "9", 2: "1", 3: "7", 4: "6", 5: None, 6: None, 7: None}
H = 512


def tile(path, label):
    img = Image.new("RGB", (H, H + 28), (24, 26, 30))
    if path and path.exists():
        src = Image.open(path).convert("RGB")
        src.thumbnail((H, H))
        img.paste(src, ((H - src.width) // 2, 28 + (H - src.height) // 2))
    ImageDraw.Draw(img).text((8, 6), label, fill=(235, 235, 235))
    return img


def summary(report_path):
    if not report_path.exists():
        return "no report"
    r = json.loads(report_path.read_text())
    return f"violations {len(r['joint_limit_violations'])}, penetrations {len(r['penetrations'])}"


runs = sys.argv[1:]
out = ROOT / "runs" / "compare"
out.mkdir(parents=True, exist_ok=True)
for n, ref in REFS.items():
    tiles = [tile(ROOT / "refs" / f"{ref}.png" if ref else None, f"T{n} reference" if ref else f"T{n} (no photo)")]
    for run in runs:
        d = ROOT / "runs" / run
        tiles.append(tile(d / f"T{n}.png", f"{run}: {summary(d / f'T{n}.report.json')}"))
    sheet = Image.new("RGB", (H * len(tiles), H + 28))
    for i, t in enumerate(tiles):
        sheet.paste(t, (i * H, 0))
    sheet.save(out / f"T{n}.png")
print(out)
