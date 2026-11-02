#!/bin/bash
# scroll, scroll_0..2: showcase/scroll-stopper ("snap to switch worlds"). One motion clip; each of its
# 8 segments is rendered as frames in its own look/backdrop (segs.json, at (i+0.25)/30 s, as its
# render.sh), composited by its comp.py (square crop, flash, punch-zoom, title), then cut:
#   scroll = frames 0..164 (5.5 s); scroll_0 / _1 / _2 = 66 frames from frame 0 / 48 / 96.
source "$(dirname "$0")/common.sh"
S="$H/showcase/scroll-stopper"; D="$W/scroll"; mkdir -p "$D"
cp "$S/clip.json" "$S/segs.json" "$S/comp.py" "$D/"
cd "$D"; rm -rf raw comp
python -c "
import json
for k,(a,b,look,bg,sp) in enumerate(json.load(open('segs.json'))):
    print(k, json.dumps(look,separators=(',',':')), bg, sp, ' '.join(f'{(i+0.25)/30:.4f}s' for i in range(a,b)))
" > jobs.txt
s0=$(date +%s)
while read -r k look bg sp times; do
  "$P" --clip clip.json --assets "$A" frame $times --look-json "$look" --background "$bg" --spotlight "$sp" -o raw/s$k > "seg$k.log" 2>&1
  log "scroll segment $k rendered"
done < jobs.txt
log "  frames done in $(( $(date +%s) - s0 )) s"
python comp.py > /dev/null
C709="-colorspace bt709 -color_primaries bt709 -color_trc bt709"
sq() { ffmpeg -v error -y -framerate 30 -start_number "$2" -i comp/%04d.png -frames:v "$3" $(enc) $C709 "$W/_$1.mp4"
       install_asset "$1" "$W/_$1.mp4"; }
sq scroll 0 165
sq scroll_0 0 66
sq scroll_1 48 66
sq scroll_2 96 66
