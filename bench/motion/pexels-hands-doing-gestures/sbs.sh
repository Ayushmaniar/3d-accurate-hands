#!/bin/bash
# sbs.sh out.png start step count : grid of [ref|render] pairs from out.mp4, 3 pairs per row
export PATH="/c/Users/Rajat/tools/ffmpeg-n9.0-latest-win64-gpl-shared-9.0/bin:$PATH"
R="/c/Users/Rajat/Documents/3D Accurate Hands/Reference videos to Animate/clean/pexels-hands-doing-gestures/seg01.mp4"
o=$1; s=$2; st=$3; n=$4; mkdir -p sb; rm -f sb/*; ins=()
for ((i=0;i<n;i++)); do t=$(python -c "print(round($s+$i*$st,3))")
ffmpeg -loglevel error -y -ss $t -i "$R" -frames:v 1 -vf scale=320:180 sb/a.png
ffmpeg -loglevel error -y -ss $t -i out.mp4 -frames:v 1 -vf scale=320:180 sb/b.png
ffmpeg -loglevel error -y -i sb/a.png -i sb/b.png -filter_complex "[0:v]drawtext=text='$t':x=4:y=4:fontsize=18:fontcolor=yellow[a];[a][1:v]hstack,pad=iw+6:ih+4:0:0:black" sb/p_$i.png 2>/dev/null
ins+=(-i sb/p_$i.png); done
ffmpeg -loglevel error -y "${ins[@]}" -filter_complex "xstack=inputs=$n:grid=3x$(( (n+2)/3 ))" $o
