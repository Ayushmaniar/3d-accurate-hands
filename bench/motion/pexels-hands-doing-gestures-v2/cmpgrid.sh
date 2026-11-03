#!/bin/bash
# usage: cmpgrid.sh out.png t1 t2 ... (pairs ref over ours, 4 per row)
export PATH="$HOME/tools/ffmpeg-n9.0-latest-win64-gpl-shared-9.0/bin:$PATH"
REF="../../../Reference videos to Animate/clean/pexels-hands-doing-gestures/seg01.mp4"
OUT=$1; shift
mkdir -p cg; rm -f cg/*.png
i=0
for t in "$@"; do
  ffmpeg -v error -y -ss $t -i "$REF" -frames:v 1 -vf "scale=400:225,drawtext=text='ref $t':x=5:y=5:fontcolor=yellow:fontsize=18" cg/a$i.png 2>/dev/null
  ffmpeg -v error -y -ss $t -i out.mp4 -frames:v 1 -vf "scale=400:225,drawtext=text='ours $t':x=5:y=5:fontcolor=yellow:fontsize=18" cg/b$i.png 2>/dev/null
  ffmpeg -v error -y -i cg/a$i.png -i cg/b$i.png -filter_complex vstack cg/p$(printf %02d $i).png
  i=$((i+1))
done
ffmpeg -v error -y -i cg/p%02d.png -vf tile=4x$(( (i+3)/4 )) -frames:v 1 $OUT
