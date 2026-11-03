#!/bin/bash
# usage: cmp3.sh out.png t1 t2 ...  -> one row per time: reference | render | 50% overlay
export PATH="$HOME/tools/ffmpeg-n9.0-latest-win64-gpl-shared-9.0/bin:$PATH"
D="$(git rev-parse --show-toplevel)/bench/motion/pexels-hands-doing-gestures"
R="$(git rev-parse --show-toplevel)/Reference videos to Animate/clean/pexels-hands-doing-gestures/seg01.mp4"
F="$(git rev-parse --show-toplevel)/starters/fframes-clip/target/release/fframes-clip.exe"
A="$(git rev-parse --show-toplevel)/assets"
BG="${BG:-#5d5c60}"
out=$1; shift
cd "$D"; rm -rf cmpf; mkdir -p cmpf
times=$(IFS=,; echo "$*" | sed 's/\([0-9.]\+\)/\1s/g')
"$F" --clip ${CLIP:-clip.json} --assets "$A" --background "$BG" ${SPOT:+--spotlight $SPOT} frame "$times" -o cmpf/r --scale 0.25 >/dev/null 2>cmpf/log.txt || cat cmpf/log.txt
i=0; rows=()
for t in "$@"; do
  ffmpeg -loglevel error -y -ss $t -i "$R" -frames:v 1 -vf "scale=480:270" cmpf/ref_$i.png
  ffmpeg -loglevel error -y -i cmpf/ref_$i.png -i "cmpf/r/${t}s.png" -filter_complex "[0:v]split[a][a2];[1:v]split[b][b2];[a2][b2]blend=all_mode=average[c];[a]drawtext=text='$t':x=6:y=6:fontsize=22:fontcolor=yellow[a3];[a3][b][c]hstack=3" cmpf/row_$i.png
  rows+=(-i cmpf/row_$i.png)
  i=$((i+1))
done
if (( i == 1 )); then cp cmpf/row_0.png "$out"; else
ffmpeg -loglevel error -y "${rows[@]}" -filter_complex "vstack=inputs=$i" "$out"; fi
