# Writes captions.txt, the ffmpeg filter that captions hands.mp4: each letter appears as its
# handshape lands (gold while it is the current letter, then white), then "I LOVE YOU".
from PIL import ImageFont

FONT = "C:/Windows/Fonts/segoeuib.ttf"
F = FONT.replace(":", r"\:")
SIZE, TRACK = 112, 22
letters = [("H", 0.6), ("A", 1.5), ("N", 2.2), ("D", 2.9), ("K", 3.6), ("I", 4.3), ("T", 5.0)]
lead = 0.12          # caption shows as the shape forms
t_ily = 5.95
end = 7.5
ACC, WHITE = "0xFFD27A", "0xF4F1EC"

font = ImageFont.truetype(FONT, SIZE)
widths = [font.getlength(c) for c, _ in letters]
x = 960 - (sum(widths) + TRACK * (len(widths) - 1)) / 2
centers = []
for w in widths:
    centers.append(round(x + w / 2))
    x += w + TRACK


def fade(t0, dur=0.12):
    return rf"alpha='min(1\,(t-{t0})/{dur})'"


parts = []
for i, (ch, t) in enumerate(letters):
    t0 = t - lead
    t1 = (letters[i + 1][1] - lead) if i + 1 < len(letters) else 5.75
    common = (f"fontfile='{F}':text='{ch}':fontsize={SIZE}:x={centers[i]}-text_w/2:y=800:"
              "shadowcolor=black@0.6:shadowx=0:shadowy=3")
    parts.append(rf"drawtext={common}:fontcolor={ACC}:enable='between(t\,{t0}\,{t1})':{fade(t0)}")
    parts.append(rf"drawtext={common}:fontcolor={WHITE}:enable='gte(t\,{t1})'")
parts.append(rf"drawtext=fontfile='{F}':text='I LOVE YOU':fontsize=58:x=(w-text_w)/2:y=950:"
             rf"fontcolor={ACC}:shadowcolor=black@0.6:shadowx=0:shadowy=3:"
             rf"enable='gte(t\,{t_ily})':{fade(t_ily, 0.25)}")
parts.append("noise=alls=3:allf=t")  # dither: hides banding in the spotlight
parts.append(f"fade=t=in:st=0:d=0.25,fade=t=out:st={end - 0.35}:d=0.35")

f = [
    rf"color=black:s=1920x420:d={end},format=rgba,geq=r=0:g=0:b=0:a='215*pow(Y/H\,1.6)'[grad]",
    "[0:v][grad]overlay=0:660[bg]",
    "[bg]" + ",".join(parts) + "[v]",
]
open("captions.txt", "w").write(";\n".join(f))
