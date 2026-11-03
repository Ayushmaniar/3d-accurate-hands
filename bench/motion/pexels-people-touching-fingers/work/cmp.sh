#!/bin/bash
# usage: cmp.sh clip.json out.png t1 t2 ...
export PATH="$HOME/tools/ffmpeg-n9.0-latest-win64-gpl-shared-9.0/bin:$PATH"
CLIP=$1; OUT=$2; shift 2
REF="$(git rev-parse --show-toplevel)/Reference videos to Animate/clean/pexels-people-touching-fingers/seg01.mp4"
P="$(git rev-parse --show-toplevel)/starters/fframes-clip/target-portrait/release/fframes-clip.exe"
mkdir -p refc mine
rm -f mine/*.png
for t in "$@"; do
  [ -f refc/$t.png ] || ffmpeg -loglevel error -y -ss $t -i "$REF" -frames:v 1 refc/$t.png
done
TS=$(printf "%ss," "$@"); TS=${TS%,}
"$P" --clip $CLIP --assets "$(git rev-parse --show-toplevel)/assets" --background "${BG:-#120605}" --spotlight "${SPOT:-0.2745,0.50,0.22,#dcaaa6}" frame "$TS" -o mine >/dev/null 2>&1
python cmp.py "$OUT" "$@"
