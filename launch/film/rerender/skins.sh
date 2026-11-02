#!/bin/bash
# skin_light..skin_dark: the Pexels-gestures clip (bench v2, clip.json) 4..8 s, realistic, five skins
# (as launch/film/work/renders/skins.sh). Assets are straight copies.
source "$(dirname "$0")/common.sh"
C="$H/bench/motion/pexels-hands-doing-gestures-v2/clip.json"
mkdir -p "$W/skins"
for s in light medium tan brown dark; do
  render "$C" "$W/skins/skin_$s.mp4" --look-json "{\"style\":\"realistic\",\"skin\":\"$s\"}" \
    --background '#17191d' --spotlight '0.5,0.45,0.42,#3a3f48' render 4s..8s
  cut "skin_$s" "$W/skins/skin_$s.mp4" 0 120
done
