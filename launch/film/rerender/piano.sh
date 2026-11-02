#!/bin/bash
# piano: showcase/piano. The hands on a flat #2a1450 backdrop, keyed out and composited with a soft
# drop shadow over bg.png (the halftone keyboard bg.py draws with the clip's camera), 6 s.
source "$(dirname "$0")/common.sh"
S="$H/showcase/piano"; D="$W/piano"; mkdir -p "$D"
render "$S/clip.json" "$D/hands.mp4" --background "#2a1450" render
ffmpeg -v error -y -loop 1 -framerate 30 -i "$S/bg.png" -i "$D/hands.mp4" -filter_complex "[1]format=rgba,colorkey=0x2a1450:0.07:0.04,split[h][m];[m]alphaextract,boxblur=18,lut=y=val*0.5[sh];color=black:s=1920x1080:r=30,format=rgba[blk];[blk][sh]alphamerge[shadow];[0][shadow]overlay=0:26:shortest=1[b2];[b2][h]overlay=0:0:shortest=1,format=yuv420p" \
  -frames:v 180 $(enc) "$W/_piano.mp4"
install_asset piano "$W/_piano.mp4"
