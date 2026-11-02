import cv2, numpy as np
cap=cv2.VideoCapture(r"C:\Users\Rajat\Documents\3D Accurate Hands\Reference videos to Animate\clean\pexels-people-touching-fingers\seg01.mp4")
frames=[]
while True:
    ok,f=cap.read()
    if not ok: break
    frames.append(cv2.cvtColor(f,cv2.COLOR_BGR2GRAY))
fps=29.97
ref=frames[int(3.0*fps)]
T={'top':(470,560,300,320),'bot':(250,840,320,240)}
for k,(x,y,w,h) in T.items():
    tpl=ref[y:y+h,x:x+w]
    prev=(0,0)
    for i in range(int(1.3*fps),-1,-3):
        f=frames[i]
        px,py=x+prev[0],y+prev[1]
        sx,sy=max(0,px-150),max(0,py-150)
        win=f[sy:min(1920,py+h+150), sx:min(1080,px+w+150)]
        best=None
        for ang in range(-40,41,5):
            M=cv2.getRotationMatrix2D((w/2,h/2),ang,1.0)
            t2=cv2.warpAffine(tpl,M,(w,h),borderMode=cv2.BORDER_REPLICATE)
            if win.shape[0]<h or win.shape[1]<w: continue
            r=cv2.matchTemplate(win,t2,cv2.TM_CCOEFF_NORMED)
            _,mv,_,ml=cv2.minMaxLoc(r)
            if best is None or mv>best[0]: best=(mv,ml,ang)
        mv,ml,ang=best
        dx,dy=sx+ml[0]-x, sy+ml[1]-y
        prev=(dx,dy)
        print(f"{k} {i/fps:.2f} centre ({(x+dx+w/2)/1080:.3f},{(y+dy+h/2)/1920:.3f}) d ({dx/1080:+.3f},{dy/1920:+.3f}) rot {ang:+d} score {mv:.2f}")
