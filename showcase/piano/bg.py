"""Backdrop for the piano clip: purple, magenta spotlight, a faint Spider-Verse keyboard
plane drawn with the clip's own camera at the height the fingertips strike (y = -0.10)."""
import json, math
import numpy as np
from PIL import Image, ImageDraw, ImageFilter

W, H = 1920, 1080
clip = json.load(open("clip.json"))
cam = clip["camera"]
C = np.array(cam["position"]); T = np.array(cam["target"])
f = T - C; f /= np.linalg.norm(f)
r = np.cross(f, [0, 1, 0]); r /= np.linalg.norm(r)
u = np.cross(r, f)
fy = (H / 2) / math.tan(math.radians(cam["fov"]) / 2)


def proj(p):
    d = np.array(p) - C
    z = d @ f
    return (W / 2 + fy * (d @ r) / z, H / 2 - fy * (d @ u) / z)


# base: purple + magenta spotlight
yy, xx = np.mgrid[0:H, 0:W]
d = np.sqrt((xx - W * 0.5) ** 2 + (yy - H * 0.45) ** 2) / (H * 0.62)
g = np.clip(1 - d, 0, 1) ** 1.6
base = np.array([42, 20, 80]) * (1 - g[..., None]) + np.array([192, 48, 106]) * g[..., None]
img = Image.fromarray(base.astype(np.uint8)).convert("RGBA")

# keyboard plane
KY, Z0, Z1 = -0.108, -0.175, 0.02          # key top height, front edge, back
WK = 0.0235                                 # white key width
lay = Image.new("RGBA", (W, H), (0, 0, 0, 0))
dr = ImageDraw.Draw(lay)
n0, n1 = -16, 16
quad = [proj([n0 * WK, KY, Z0]), proj([n1 * WK, KY, Z0]), proj([n1 * WK, KY, Z1]), proj([n0 * WK, KY, Z1])]
dr.polygon(quad, fill=(255, 220, 240, 28))
for i in range(n0, n1 + 1):                 # white key seams
    dr.line([proj([i * WK, KY, Z0]), proj([i * WK, KY, Z1])], fill=(255, 230, 245, 70), width=2)
black = {1, 2, 4, 5, 6}                     # black keys after these whites in an octave (C=0)
for i in range(n0, n1):
    if (i % 7) in black:
        x = i * WK
        q = [proj([x - 0.0065, KY + 0.004, Z0 + 0.06]), proj([x + 0.0065, KY + 0.004, Z0 + 0.06]),
             proj([x + 0.0065, KY + 0.004, Z1]), proj([x - 0.0065, KY + 0.004, Z1])]
        dr.polygon(q, fill=(20, 8, 45, 85))
# front edge: a bright ink line with cyan / magenta misregistered plates
fe = [proj([n0 * WK, KY, Z0]), proj([n1 * WK, KY, Z0])]
for off, col in ((-4, (0, 230, 255, 120)), (4, (255, 40, 160, 120)), (0, (255, 240, 250, 150))):
    dr.line([(fe[0][0], fe[0][1] + off), (fe[1][0], fe[1][1] + off)], fill=col, width=3)
# fade the plane toward the back
fade = Image.fromarray((np.clip((yy - quad[3][1]) / (quad[0][1] - quad[3][1] + 1e-6), 0, 1) ** 0.7 * 255).astype(np.uint8))
lay.putalpha(Image.fromarray((np.array(lay.getchannel("A"), float) * np.array(fade, float) / 255).astype(np.uint8)))
img = Image.alpha_composite(img, lay)

# halftone dots over the whole backdrop (Spider-Verse print feel)
dots = Image.new("L", (W, H), 0)
dd = ImageDraw.Draw(dots)
S = 14
for y in range(0, H, S):
    for x in range((y // S % 2) * S // 2, W, S):
        rr = 1.0 + 2.6 * (y / H)
        dd.ellipse([x - rr, y - rr, x + rr, y + rr], fill=40)
img = Image.composite(Image.new("RGBA", (W, H), (255, 120, 200, 255)), img, dots)
img.convert("RGB").save("bg.png")
print("front edge y px:", [round(p[1]) for p in fe], "back y px:", round(quad[3][1]))
