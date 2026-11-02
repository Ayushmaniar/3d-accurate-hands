#!/bin/bash
# a1_hk: showcase/bw-burrows (gen.py -> clip.json, 24 fps, 12 s). Render, the showcase's B&W grade
# (centre crop, grey, edge fades, vignette, grain; 24 fps), then the film's projector look (flicker,
# grain) at 30 fps, first 11.2 s. CRF 24 as the film build chose: the grain makes CRF 17 ~3x bigger.
source "$(dirname "$0")/common.sh"
S="$H/showcase/bw-burrows"; D="$W/bw-burrows"; mkdir -p "$D"
render "$S/clip.json" "$D/raw.mp4" --background "#121212" --spotlight 0.5,0.68,0.8,#565656 render
G="crop=1440:1080:240:0,deband=1thr=0.03:range=24:blur=1,format=gray,geq=lum='lum(X,Y)*clip((Y/H-0.10)/0.26,0,1)*clip(X/W/0.2,0,1)*clip((1-X/W)/0.2,0,1)',eq=contrast=1.25:brightness=-0.02:gamma=0.95,vignette=PI/5,noise=alls=12:allf=t,format=yuv420p"
ffmpeg -v error -y -i "$D/raw.mp4" -vf "$G" -r 24 -t 12 -c:v libx264 -crf 18 -preset slow -movflags +faststart "$D/out.mp4"
FILM="hue=s=0,eq=brightness='0.03*sin(2*PI*t*6.7)*sin(2*PI*t*1.9)+0.035*(random(1)-0.5)':contrast=1.06:eval=frame,noise=c0s=13:c0f=t+u,fps=30"
cut a1_hk "$D/out.mp4" 0 336 "$FILM" 24
