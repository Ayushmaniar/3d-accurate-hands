"""Composes the skin review sheet (docs/m5-skin.png): the reference photos over our hand in the
same three views, then close-ups.

Usage: python tools/skin/sheet.py OUT.png --refs back.png palm.png side.png
           --ours back.png palm.png side.png --closeups a.png b.png [--labels ...]
The views are portrait (2:3); the close-ups square and may be transparent (laid on grey).
"""
import argparse

from PIL import Image, ImageDraw, ImageFont

W, H = 400, 600
BAR = 26
GREY = (118, 121, 125)


def font():
    for name in ["segoeui.ttf", "arial.ttf", "DejaVuSans.ttf"]:
        try:
            return ImageFont.truetype(name, 15)
        except OSError:
            pass
    return ImageFont.load_default()


def tile(path, w, h, fit=False):
    im = Image.open(path).convert("RGBA")
    if fit:
        # A transparent render: crop to what is drawn, with a margin, and fit it in the frame.
        box = im.getchannel("A").point(lambda a: 255 if a > 8 else 0).getbbox()
        if box:
            m = int(0.06 * max(box[2] - box[0], box[3] - box[1]))
            im = im.crop((max(box[0] - m, 0), max(box[1] - m, 0), min(box[2] + m, im.width), min(box[3] + m, im.height)))
        s = min(w / im.width, h / im.height)
        im = im.resize((round(im.width * s), round(im.height * s)), Image.LANCZOS)
        out = Image.new("RGB", (w, h), GREY)
        out.paste(im, ((w - im.width) // 2, (h - im.height) // 2), im)
        return out
    # Fill the frame, cropping the longer side.
    s = max(w / im.width, h / im.height)
    im = im.resize((round(im.width * s), round(im.height * s)), Image.LANCZOS)
    x, y = (im.width - w) // 2, (im.height - h) // 2
    im = im.crop((x, y, x + w, y + h))
    out = Image.new("RGB", (w, h), GREY)
    out.paste(im, (0, 0), im)
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--refs", nargs=3, required=True)
    ap.add_argument("--ours", nargs=3, required=True)
    ap.add_argument("--closeups", nargs="+", required=True)
    ap.add_argument("--labels", nargs="+", default=[])
    a = ap.parse_args()
    f = font()
    n = len(a.closeups)
    cw = 3 * W // n
    rows = [(BAR + H), (BAR + H), (BAR + cw)]
    sheet = Image.new("RGB", (3 * W, sum(rows)), (40, 42, 46))
    d = ImageDraw.Draw(sheet)
    labels = a.labels + [""] * 20
    names = ["back", "palm", "thumb side"]
    y = 0
    for r, (paths, what) in enumerate([(a.refs, "reference photo"), (a.ours, "Handkit M5 skin (three.js preview, studio light)")]):
        for i, p in enumerate(paths):
            d.text((i * W + 8, y + 5), f"{what}: {names[i]}" if r == 0 else f"{names[i]}: {what}" if i == 0 else names[i], fill=(225, 225, 225), font=f)
            sheet.paste(tile(p, W, H), (i * W, y + BAR))
        y += BAR + H
    for i, p in enumerate(a.closeups):
        d.text((i * cw + 8, y + 5), labels[i] or f"close-up {i + 1}", fill=(225, 225, 225), font=f)
        sheet.paste(tile(p, cw, cw, fit=True), (i * cw, y + BAR))
    sheet.save(a.out, optimize=True)
    print(f"wrote {a.out} ({sheet.width}x{sheet.height})")


if __name__ == "__main__":
    main()
