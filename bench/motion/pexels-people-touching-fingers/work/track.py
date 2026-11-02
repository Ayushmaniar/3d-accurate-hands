import cv2, numpy as np
cap=cv2.VideoCapture(r"C:\Users\Rajat\Documents\3D Accurate Hands\Reference videos to Animate\clean\pexels-people-touching-fingers\seg01.mp4")
frames=[]
while True:
    ok,f=cap.read()
    if not ok: break
    frames.append(cv2.cvtColor(f,cv2.COLOR_BGR2GRAY))
fps=29.97
print(len(frames))
ref=frames[int(3.0*fps)]
T={'top':(470,560,300,320),'bot':(250,840,320,240)}  # x,y,w,h
res={}
for k,(x,y,w,h) in T.items():
    tpl=ref[y:y+h,x:x+w]
    out=[]
    for i in range(int(1.2*fps),len(frames)):
        f=frames[i]
        sx,sy=max(0,x-120),max(0,y-120)
        win=f[sy:y+h+120, sx:x+w+120]
        r=cv2.matchTemplate(win,tpl,cv2.TM_CCOEFF_NORMED)
        _,mv,_,ml=cv2.minMaxLoc(r)
        out.append((i/fps,(sx+ml[0]-x)/1080,(sy+ml[1]-y)/1920,mv))
    res[k]=out
for j in range(0,len(res['top']),3):
    a=res['top'][j]; b=res['bot'][j]
    print(f"{a[0]:.2f} top dx {a[1]:+.4f} dy {a[2]:+.4f} ({a[3]:.2f})   bot dx {b[1]:+.4f} dy {b[2]:+.4f} ({b[3]:.2f})")
