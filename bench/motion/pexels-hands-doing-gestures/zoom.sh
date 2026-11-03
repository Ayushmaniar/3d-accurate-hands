#!/bin/bash
# zoom.sh out.png t1 t2 ... : full-res reference frames at 960x540, two per row
export PATH="$HOME/tools/ffmpeg-n9.0-latest-win64-gpl-shared-9.0/bin:$PATH"
R="$(git rev-parse --show-toplevel)/Reference videos to Animate/clean/pexels-hands-doing-gestures/seg01.mp4"
out=$1; shift; i=0; ins=()
mkdir -p zf
for t in "$@"; do ffmpeg -loglevel error -y -ss $t -i "$R" -frames:v 1 -vf "crop=${CROP:-1920:1080:0:0},scale=${ZW:-960}:-2" zf/z_$i.png; ins+=(-i zf/z_$i.png); i=$((i+1)); done
if ((i==1)); then cp zf/z_0.png $out; elif ((i==2)); then ffmpeg -loglevel error -y "${ins[@]}" -filter_complex hstack $out; else
ffmpeg -loglevel error -y "${ins[@]}" -filter_complex "xstack=inputs=$i:grid=2x$(( (i+1)/2 ))" $out; fi
