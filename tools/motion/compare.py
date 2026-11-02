"""Reference vs recreation, for reviewing the motion gate.

    python tools/motion/compare.py <reference.mp4> <recreation.mp4> <out_prefix> [-n 10]

Writes <out_prefix>.sheet.png (the two videos in rows at the same times) and
<out_prefix>.side.mp4 (both side by side at the same height, the shorter one padded). Needs
ffmpeg on PATH and Pillow.
"""
import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image, ImageDraw


def probe(path):
    out = subprocess.run(
        ["ffprobe", "-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height:format=duration", "-of", "json", str(path)],
        capture_output=True, text=True, check=True,
    ).stdout
    d = json.loads(out)
    return float(d["format"]["duration"]), d["streams"][0]["width"], d["streams"][0]["height"]


def frame(path, t, height, tmp):
    f = Path(tmp) / f"f{abs(hash((str(path), t)))}.png"
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-ss", f"{t:.3f}", "-i", str(path), "-frames:v", "1", "-vf", f"scale=-2:{height}", str(f)], check=True)
    return Image.open(f).convert("RGB")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("reference")
    ap.add_argument("recreation")
    ap.add_argument("out")
    ap.add_argument("-n", type=int, default=10)
    ap.add_argument("--height", type=int, default=240)
    a = ap.parse_args()
    d_ref, _, _ = probe(a.reference)
    d_rec, _, _ = probe(a.recreation)
    dur = min(d_ref, d_rec)
    times = [dur * (i + 0.5) / a.n for i in range(a.n)]
    with tempfile.TemporaryDirectory() as tmp:
        rows = [[frame(p, t, a.height, tmp) for t in times] for p in (a.reference, a.recreation)]
    w = max(im.width for row in rows for im in row)
    sheet = Image.new("RGB", (w * a.n, a.height * 2 + 20), (30, 32, 36))
    g = ImageDraw.Draw(sheet)
    for r, row in enumerate(rows):
        for i, im in enumerate(row):
            sheet.paste(im, (i * w + (w - im.width) // 2, r * a.height))
    for i, t in enumerate(times):
        g.text((i * w + 6, a.height * 2 + 4), f"{t:.2f} s", fill=(230, 230, 230))
    g.text((6, 4), "reference", fill=(255, 255, 255))
    g.text((6, a.height + 4), "recreation", fill=(255, 255, 255))
    sheet.save(f"{a.out}.sheet.png")
    h = 720
    subprocess.run(
        ["ffmpeg", "-v", "error", "-y", "-i", a.reference, "-i", a.recreation, "-filter_complex",
         f"[0:v]fps=30,scale=-2:{h},setsar=1[a];[1:v]fps=30,scale=-2:{h},setsar=1[b];[a][b]hstack=inputs=2:shortest=1[v]",
         "-map", "[v]", "-an", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "20", f"{a.out}.side.mp4"],
        check=True,
    )
    print(f"wrote {a.out}.sheet.png and {a.out}.side.mp4 ({dur:.1f} s compared)")


if __name__ == "__main__":
    sys.exit(main())
