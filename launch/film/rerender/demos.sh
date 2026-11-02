#!/bin/bash
# holding_hands, four_hands, handshake, high_five: clips/demo_*.json (as launch/renders/demo_*.mp4),
# realistic, arms faded, motion blur 4. Straight copies of the whole render.
source "$(dirname "$0")/common.sh"
D="$W/demos"; mkdir -p "$D"
for d in holding_hands:162 four_hands:174 handshake:162 high_five:90; do
  name=${d%%:*}; n=${d#*:}
  render "$H/clips/demo_$name.json" "$D/demo_$name.mp4" --background "#1b1d22" --spotlight 0.5,0.45,0.42,#30343c \
    --look-json '{"style":"realistic","arm":"fade","motion_blur":4}' render
  cut "$name" "$D/demo_$name.mp4" 0 "$n"
done
