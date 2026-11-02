#!/bin/bash
# bench/motion/pexels-hands-doing-gestures-v2, backdrop #5d5c60:
#   clip_realistic.json -> a2_hk (from frame 30 = 1.0 s, 8.2 s)
#   clip.json (default look) -> a2p_hk (0 s, 8.2 s) and a1p_hk (from frame 218 = 7.267 s to the end,
#     with the public cut's B&W film look; CRF 24 as the film build chose, for the grain)
source "$(dirname "$0")/common.sh"
B="$H/bench/motion/pexels-hands-doing-gestures-v2"; D="$W/gestures-v2"; mkdir -p "$D"
render "$B/clip_realistic.json" "$D/out_realistic.mp4" --background "#5d5c60" render
cut a2_hk "$D/out_realistic.mp4" 30 246
render "$B/clip.json" "$D/out.mp4" --background "#5d5c60" render
cut a2p_hk "$D/out.mp4" 0 246
FILMP="fps=30,hue=s=0,eq=brightness='-0.03+0.03*sin(2*PI*t*6.7)*sin(2*PI*t*1.9)+0.035*(random(1)-0.5)':contrast=1.18:eval=frame,vignette=angle=PI/3.4,noise=c0s=13:c0f=t+u"
cut a1p_hk "$D/out.mp4" 218 328 "eq=gamma=0.88,$FILMP" 24
