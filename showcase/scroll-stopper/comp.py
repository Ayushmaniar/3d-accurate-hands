import json, os, numpy as np, wave
from PIL import Image, ImageDraw, ImageFont, ImageFilter
FPS=30; N=210; S=1080; X0=420
segs=json.load(open('segs.json'))
cuts=[a for a,*_ in segs[1:]]
os.makedirs('comp',exist_ok=True)
big=ImageFont.truetype('C:/Windows/Fonts/seguibl.ttf',150)
small=ImageFont.truetype('C:/Windows/Fonts/seguisb.ttf',46)
def x0(i):
    p=min(1,max(0,(i-159)/24)); p=p*p*(3-2*p); return int(340+(420-340)*p)
def seg_of(i):
    for k,(a,b,*_) in enumerate(segs):
        if a<=i<b: return k
for i in range(N):
    k=seg_of(i)
    im=Image.open(f'raw/s{k}/{(i+0.25)/30:.4f}s.png').convert('RGB').crop((x0(i),0,x0(i)+S,S))
    j=min([i-c for c in cuts if i>=c] or [99])
    if j<7:
        z=1+0.09*(1-j/7)**2; w=S/z; o=(S-w)/2
        im=im.crop((o,o,o+w,o+w)).resize((S,S),Image.LANCZOS)
        a={0:0.6,1:0.28,2:0.1}.get(j,0)
        if a: im=Image.blend(im,Image.new('RGB',(S,S),'white'),a)
    if i>=168:
        p=min(1,(i-168)/8); e=1-(1-p)**3
        layer=Image.new('RGBA',(S,S),(0,0,0,0)); d=ImageDraw.Draw(layer)
        t='HANDKIT'; tw=d.textlength(t,font=big); y=36+int(30*(1-e))
        d.text(((S-tw)/2,y),t,font=big,fill=(255,255,255,int(255*e)))
        p2=min(1,max(0,(i-176)/8)); e2=1-(1-p2)**3
        t2='one hand. every world.'; tw2=d.textlength(t2,font=small)
        d.text(((S-tw2)/2,y+185+int(20*(1-e2))),t2,font=small,fill=(255,255,255,int(215*e2)))
        sh=layer.filter(ImageFilter.GaussianBlur(10)); shd=Image.new('RGBA',(S,S),(0,0,0,0)); shd.putalpha(sh.split()[3].point(lambda v:v*0.6))
        im=Image.alpha_composite(Image.alpha_composite(im.convert('RGBA'),shd),layer).convert('RGB')
    im.save(f'comp/{i:04d}.png')
# audio
sr=48000; L=int(7.0*sr); out=np.zeros(L); rng=np.random.default_rng(1)
def add(sig,t):
    s=int(t*sr); n=min(len(sig),L-s); out[s:s+n]+=sig[:n]
def bp(x,lo,hi):
    X=np.fft.rfft(x); f=np.fft.rfftfreq(len(x),1/sr); X[(f<lo)|(f>hi)]=0; return np.fft.irfft(X,len(x))
snapt=[c/FPS for c in cuts]
for n,t in enumerate(snapt):
    m=len(snapt)-1==n
    tt=np.arange(int(0.12*sr))/sr
    nz=bp(rng.standard_normal(len(tt)),1200,6000)*np.exp(-tt/0.018); nz/=np.abs(nz).max()
    add(0.55*nz,t-0.012)
    tb=np.arange(int((0.9 if m else 0.35)*sr))/sr
    f=50+90*np.exp(-tb/0.04); ph=2*np.pi*np.cumsum(f)/sr
    add((0.75 if m else 0.45+0.04*n)*np.sin(ph)*np.exp(-tb/(0.35 if m else 0.12)),t-0.012)
# riser 3.6 -> 5.3
r0,r1=3.6,5.3; tr=np.arange(int((r1-r0)*sr))/sr
rz=bp(rng.standard_normal(len(tr)),300,9000); rz/=np.abs(rz).max()
add(0.22*rz*(tr/(r1-r0))**2.5,r0)
# whoosh on hand entry
tw=np.arange(int(0.6*sr))/sr; wz=bp(rng.standard_normal(len(tw)),200,2500); wz/=np.abs(wz).max()
add(0.15*wz*np.sin(np.pi*tw/0.6)**2,0.0)
out=out/np.abs(out).max()*0.89
with wave.open('audio.wav','wb') as w:
    w.setnchannels(1); w.setsampwidth(2); w.setframerate(sr); w.writeframes((out*32767).astype(np.int16).tobytes())
print('ok')
