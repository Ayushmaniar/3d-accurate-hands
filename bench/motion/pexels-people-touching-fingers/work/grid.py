import sys
from PIL import Image, ImageDraw
im=Image.open(sys.argv[1]).convert('RGB').resize((540,960)); d=ImageDraw.Draw(im)
for i in range(1,20):
    c=(0,255,0) if i%2==0 else (0,110,0)
    d.line([(0,i*48),(540,i*48)],fill=c); d.line([(i*27,0),(i*27,960)],fill=c)
    if i%2==0: d.text((2,i*48+1),f"{i*5}",fill=(255,255,0)); d.text((i*27+1,2),f"{i*5}",fill=(255,255,0))
im.save(sys.argv[2])
