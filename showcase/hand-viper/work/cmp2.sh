#!/bin/sh
# Step-matched sheet: reference frame (top) vs out.mp4 (bottom) for steps 1-7.
FF=/c/Users/Rajat/tools/ffmpeg-n9.0-latest-win64-gpl-shared-9.0/bin
REF="/c/Users/Rajat/Documents/3D Accurate Hands/Reference videos to Animate/raw/hand-viper-tutorial.mp4"
OUT="$(dirname "$0")/../out.mp4"
T=$(mktemp -d)
i=0
for pair in 0.6:1.0 4.6:2.0 7.6:3.0 12.3:4.0 19.6:5.0 21.0:6.0 25.3:7.0; do
  i=$((i+1)); r=${pair%%:*}; m=${pair##*:}
  "$FF/ffmpeg" -v error -y -ss $r -i "$REF" -frames:v 1 -vf "crop=1080:1080:0:300,scale=270:270,pad=480:270:105:0:color=0x1b1d22" $T/r$i.png
  "$FF/ffmpeg" -v error -y -ss $m -i "$OUT" -frames:v 1 -vf "scale=480:270" $T/m$i.png
  "$FF/ffmpeg" -v error -y -i $T/r$i.png -i $T/m$i.png -filter_complex vstack $T/c$i.png
done
"$FF/ffmpeg" -v error -y -i $T/c%d.png -vf tile=7x1 -frames:v 1 "$(dirname "$0")/compare_steps_v2.png"
rm -rf $T
