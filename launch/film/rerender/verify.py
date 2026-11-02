"""Compare each re-rendered asset in launch/film/assets/v with the previous one kept in
launch/film/work/old_assets (the newest backup of that name): size, fps, frame count, duration,
and the mean absolute difference (0-255 grey) at 25 / 50 / 75 % of the clip plus over all frames
at 96x54. Writes old | new | difference sheets for clips that differ to work/rerender/verify/.

    python launch/film/rerender/verify.py [name ...]
"""
import glob, json, os, subprocess, sys
import numpy as np
from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
V = os.path.join(ROOT, "assets", "v")
OLD = os.path.join(ROOT, "work", "old_assets")
OUT = os.path.join(ROOT, "work", "rerender", "verify")
NAMES = ["arcane_brown", "spider_medium", "toon_light", "clay_tan", "real_dark",
         "skin_light", "skin_medium", "skin_tan", "skin_brown", "skin_dark",
         "a1_hk", "a1p_hk", "a2_hk", "a2p_hk", "huddle", "hs_kid", "rps",
         "holding_hands", "four_hands", "handshake", "high_five", "piano",
         "scroll", "scroll_0", "scroll_1", "scroll_2", "sign", "shipit", "a3_hk"]
FLAG = 2.0   # mean |diff| (0-255) above which a clip is called out


def probe(p):
    r = subprocess.run(["ffprobe", "-v", "error", "-count_frames", "-show_entries",
                        "stream=codec_type,width,height,r_frame_rate,nb_read_frames:format=duration",
                        "-of", "json", p], capture_output=True, text=True, check=True)
    d = json.loads(r.stdout)
    v = [s for s in d["streams"] if s["codec_type"] == "video"][0]
    return dict(size=f'{v["width"]}x{v["height"]}', fps=v["r_frame_rate"], frames=int(v["nb_read_frames"]),
                dur=round(float(d["format"]["duration"]), 3),
                audio=any(s["codec_type"] == "audio" for s in d["streams"]))


def frames(p, w, h, vf=""):
    r = subprocess.run(["ffmpeg", "-v", "error", "-i", p, "-vf", f"{vf}scale={w}:{h},format=gray",
                        "-f", "rawvideo", "-"], capture_output=True, check=True)
    return np.frombuffer(r.stdout, np.uint8).reshape(-1, h, w).astype(np.float32)


def rgb_frame(p, i):
    r = subprocess.run(["ffmpeg", "-v", "error", "-i", p, "-vf", f"select=eq(n\\,{i}),scale=640:-2",
                        "-frames:v", "1", "-f", "image2pipe", "-c:v", "png", "-"], capture_output=True, check=True)
    import io
    return Image.open(io.BytesIO(r.stdout)).convert("RGB")


def old_of(name):
    c = [f for f in [os.path.join(OLD, name + ".mp4")] + glob.glob(os.path.join(OLD, name + ".*.mp4")) if os.path.isfile(f)]
    return max(c, key=os.path.getmtime) if c else None


def main():
    names = sys.argv[1:] or NAMES
    os.makedirs(OUT, exist_ok=True)
    rows = []
    for n in names:
        new, old = os.path.join(V, n + ".mp4"), old_of(n)
        if not old:
            rows.append((n, "no old copy kept (unchanged or not re-rendered)")); continue
        a, b = probe(old), probe(new)
        same = all(a[k] == b[k] for k in ("size", "fps", "frames", "audio")) and abs(a["dur"] - b["dur"]) < 0.02
        w, h = (480, round(480 * int(a["size"].split("x")[1]) / int(a["size"].split("x")[0]) / 2) * 2)
        fo, fn = frames(old, w, h), frames(new, w, h)
        m = min(len(fo), len(fn))
        idx = [int(m * q) for q in (0.25, 0.5, 0.75)]
        d3 = [float(np.abs(fo[i] - fn[i]).mean()) for i in idx]
        allm = float(np.abs(fo[:m] - fn[:m]).mean())
        per = np.abs(fo[:m] - fn[:m]).mean(axis=(1, 2))
        worst = int(per.argmax())
        flag = max(d3 + [allm]) > FLAG or not same
        if flag:
            ims = []
            for i in idx + [worst]:
                o, x = rgb_frame(old, i), rgb_frame(new, i)
                d = Image.fromarray(np.clip(np.abs(np.asarray(o, np.int16) - np.asarray(x, np.int16)) * 4, 0, 255).astype(np.uint8))
                row = Image.new("RGB", (o.width * 3, o.height)); row.paste(o, (0, 0)); row.paste(x, (o.width, 0)); row.paste(d, (o.width * 2, 0))
                ims.append(row)
            sheet = Image.new("RGB", (ims[0].width, sum(i.height for i in ims)))
            y = 0
            for i in ims: sheet.paste(i, (0, y)); y += i.height
            sheet.save(os.path.join(OUT, n + ".png"))
        rows.append((n, f'{b["size"]} {b["fps"]} {b["frames"]}f {b["dur"]}s{" +audio" if b["audio"] else ""}'
                        f'{"" if same else "  MISMATCH old " + json.dumps(a)} | diff @25/50/75%: '
                        + " ".join(f"{x:.2f}" for x in d3) + f" | all frames {allm:.2f} (worst frame {worst}: {per[worst]:.2f})"
                        + ("  <-- DIFFERS, see work/rerender/verify/" + n + ".png" if flag else "")
                        + f'  [old: {os.path.basename(old)}]'))
    for n, s in rows:
        print(f"{n:14s} {s}")


if __name__ == "__main__":
    main()
