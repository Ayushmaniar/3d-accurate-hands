#!/bin/bash
# pair.sh video.mp4 outprefix t1 t2 ... : ref (top) vs video (bottom), 4 pairs per image
export PATH="/c/Users/Rajat/tools/ffmpeg-n9.0-latest-win64-gpl-shared-9.0/bin:$PATH"
R="../../../Reference videos to Animate/clean/hands-burrows-roberts/bench_144-156.mp4"
V=$1; O=$2; shift 2; mkdir -p tmp/pp; rm -f tmp/pp/*.png; i=0
for t in "$@"; do
  ffmpeg -v error -y -ss $t -i "$R" -frames:v 1 -vf "scale=320:240,drawtext=text='$t':x=4:y=4:fontsize=18:fontcolor=yellow" tmp/pp/r.png
  ffmpeg -v error -y -ss $t -i "$V" -frames:v 1 -vf scale=320:240 tmp/pp/v.png
  ffmpeg -v error -y -i tmp/pp/r.png -i tmp/pp/v.png -filter_complex vstack tmp/pp/p$(printf %02d $i).png 2>/dev/null
  i=$((i+1))
done
n=0
for ((s=0; s<i; s+=4)); do
  ffmpeg -v error -y -start_number $s -i tmp/pp/p%02d.png -frames:v 1 -vf tile=4x1 ${O}_$n.png 2>/dev/null; n=$((n+1))
done
