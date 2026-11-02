#!/bin/bash
# usage: cmp.sh clip.json out.png t1 t2 ...
export PATH="/c/Users/Rajat/tools/ffmpeg-n9.0-latest-win64-gpl-shared-9.0/bin:$PATH"
CLIP=$1; OUT=$2; shift 2
REF="/c/Users/Rajat/Documents/3D Accurate Hands/Reference videos to Animate/clean/pexels-people-touching-fingers/seg01.mp4"
P="/c/Users/Rajat/Documents/3D Accurate Hands/starters/fframes-clip/target-portrait/release/fframes-clip.exe"
mkdir -p refc mine
rm -f mine/*.png
for t in "$@"; do
  [ -f refc/$t.png ] || ffmpeg -loglevel error -y -ss $t -i "$REF" -frames:v 1 refc/$t.png
done
TS=$(printf "%ss," "$@"); TS=${TS%,}
"$P" --clip $CLIP --assets "/c/Users/Rajat/Documents/3D Accurate Hands/assets" --background "${BG:-#120605}" --spotlight "${SPOT:-0.2745,0.50,0.22,#dcaaa6}" frame "$TS" -o mine >/dev/null 2>&1
python cmp.py "$OUT" "$@"
