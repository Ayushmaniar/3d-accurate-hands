#!/bin/sh
# The phone review video of the web targets: the same clip from fframes, Remotion and HyperFrames,
# stacked and labelled, once per look. Renders come from the three starters:
#   starters/fframes-clip/out_<look>.mp4, starters/remotion-clip/out/<look>.mp4,
#   starters/hyperframes-clip/out/<look>.mp4
# Usage: sh tools/looks/web_targets_video.sh <out.mp4> <look:name>...   e.g. demo:realistic spider:spiderverse
set -e
root="$(cd "$(dirname "$0")/../.." && pwd)"
out="$1"; shift
font="C\\:/Windows/Fonts/segoeui.ttf"
tmp="$(mktemp -d)"
i=0
for pair in "$@"; do
  key="${pair%%:*}"; name="${pair#*:}"
  ff="$root/starters/fframes-clip/out_$key.mp4"
  rm="$root/starters/remotion-clip/out/$key.mp4"
  hf="$root/starters/hyperframes-clip/out/$key.mp4"
  panel="crop=1440:1080:240:0,scale=960:720,setsar=1"
  label() { echo "drawtext=fontfile='$font':text='$1':x=28:y=22:fontsize=34:fontcolor=white@0.92,drawtext=fontfile='$font':text='$name':x=w-tw-28:y=22:fontsize=30:fontcolor=white@0.6"; }
  ffmpeg -v error -y -i "$ff" -i "$rm" -i "$hf" -filter_complex \
    "[0:v]$panel,$(label fframes)[a];[1:v]$panel,$(label Remotion)[b];[2:v]$panel,$(label HyperFrames)[c];[a][b][c]vstack=inputs=3,fps=30,format=yuv420p[v]" \
    -map "[v]" -c:v libx264 -crf 18 -preset fast "$tmp/part$i.mp4"
  echo "file 'part$i.mp4'" >> "$tmp/list.txt"
  i=$((i + 1))
done
ffmpeg -v error -y -f concat -safe 0 -i "$tmp/list.txt" -c:v libx264 -crf 28 -preset slow -pix_fmt yuv420p -movflags +faststart "$out"
rm -rf "$tmp"
ls -la "$out"
