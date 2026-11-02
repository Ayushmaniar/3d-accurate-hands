#!/bin/bash
# Re-render every Handkit-made clip of the launch film with the current binaries, one after another:
#   bash launch/film/rerender/all.sh          (or one source: bash launch/film/rerender/piano.sh)
# then compare with the previous assets (kept in work/old_assets): python launch/film/rerender/verify.py
# Needs: target/release/handkit.exe and starters/fframes-clip/target/release/fframes-clip.exe
# (cargo build --release), assets/hand_*.glb, FFmpeg 9 (path in common.sh), Python with numpy + Pillow.
# Source renders land in launch/film/work/rerender/; only ship-it renders inside its own project.
# Not here: web3 (fframes / Remotion / HyperFrames
# side by side from the starters' renders and the WASM build; see phone/ and tools/looks/web_targets_video.sh).
set -e
cd "$(dirname "$0")"
for s in looks skins gestures_v2 people demos piano sign scroll bw_burrows shipit adam; do
  echo "=== $s"; bash ./$s.sh
done
echo ALLDONE
