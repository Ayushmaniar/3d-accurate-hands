#!/bin/bash
# shipit: showcase/ship-it, its own fframes project (the starter with a release-card UI). The UI
# follows the fingertip from src/tip.json, which tip_track.py samples from the clip with handkit, so
# that is regenerated first; then the project is rebuilt and renders its out.mp4. Used from 0 s.
source "$(dirname "$0")/common.sh"
S="$H/showcase/ship-it"
cd "$S"
cp src/tip.json "$W/shipit_tip_before.json"
python tip_track.py > /dev/null
if cmp -s src/tip.json "$W/shipit_tip_before.json"; then log "ship-it: tip.json unchanged"; else log "ship-it: tip.json CHANGED (the fingertip path moved)"; fi
s0=$(date +%s)
cargo build --release > "$W/shipit_build.log" 2>&1 || { tail -30 "$W/shipit_build.log"; exit 1; }
log "ship-it built in $(( $(date +%s) - s0 )) s"
s0=$(date +%s)
./target/release/fframes-clip.exe render -o out.mp4 > "$W/shipit.log" 2>&1 || { cat "$W/shipit.log"; exit 1; }
log "ship-it rendered in $(( $(date +%s) - s0 )) s"
cut shipit "$S/out.mp4" 0 180
