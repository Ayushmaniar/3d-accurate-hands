#!/bin/bash
# adam: the Spider-Verse "Creation of Adam" (showcase/spiderverse-adam), rendered with the portrait
# player (cargo build --release --features portrait, CARGO_TARGET_DIR=target-portrait), used from
# 0.6 s for 6.27 s (188 frames).
source "$(dirname "$0")/common.sh"
P="$H/starters/fframes-clip/target-portrait/release/fframes-clip.exe"
render "$H/showcase/spiderverse-adam/clip.json" "$W/a3_hk.mp4" --background "#180a07" --spotlight "0.49,0.5,0.235,#e4aca8" render
cut a3_hk "$W/a3_hk.mp4" 18 188
