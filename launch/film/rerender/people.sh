#!/bin/bash
# The multi-person showcases, each a straight copy of its whole render:
#   huddle (showcase/huddle), hs_kid (showcase/handshake), rps (showcase/rock-paper-scissors)
source "$(dirname "$0")/common.sh"
S="$H/showcase"; D="$W/people"; mkdir -p "$D"
render "$S/huddle/clip.json" "$D/huddle.mp4" --background "#1b1d22" --spotlight 0.5,0.5,0.45,#353a45 render
cut huddle "$D/huddle.mp4" 0 240
render "$S/handshake/clip.json" "$D/hs_kid.mp4" render
cut hs_kid "$D/hs_kid.mp4" 0 180
render "$S/rock-paper-scissors/clip.json" "$D/rps.mp4" --background "#1c1d22" --spotlight 0.5,0.5,0.45,#363a47 render
cut rps "$D/rps.mp4" 0 150
