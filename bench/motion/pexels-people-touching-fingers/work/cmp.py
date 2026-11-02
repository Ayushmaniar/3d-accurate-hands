import sys, os
from PIL import Image, ImageDraw
out=sys.argv[1]; ts=sys.argv[2:]
crop=[float(v) for v in os.environ.get('CROP','0,0,1,1').split(',')]
def ld(p):
    im=Image.open(p).convert('RGB'); W,H=im.size
    return im.crop((int(crop[0]*W),int(crop[1]*H),int(crop[2]*W),int(crop[3]*H)))
r0=ld(f'refc/{ts[0]}.png'); aspect=r0.size[1]/r0.size[0]
w=int(os.environ.get('CW', 270 if len(ts)<=4 else 200)); h=int(w*aspect)
G=Image.new('RGB',(w*len(ts),h*3))
for i,t in enumerate(ts):
    r=ld(f'refc/{t}.png').resize((w,h)); m=ld(f'mine/{t}s.png').resize((w,h))
    b=Image.blend(r,m,0.5)
    for j,im in enumerate((r,m,b)): G.paste(im,(i*w,j*h))
    ImageDraw.Draw(G).text((i*w+4,4),t,fill=(255,255,0))
G.save(out)
