#!/bin/bash
# arcane_brown, spider_medium, toon_light, clay_tan, real_dark: the Pexels-gestures clip (bench v2,
# clip.json) 12..16 s in five looks (as launch/film/work/renders/run.sh). Assets are straight copies.
source "$(dirname "$0")/common.sh"
C="$H/bench/motion/pexels-hands-doing-gestures-v2/clip.json"
mkdir -p "$W/looks"
r() { render "$C" "$W/looks/$1.mp4" --look-json "$2" --background "$3" --spotlight "$4" render 12s..16s
      cut "$1" "$W/looks/$1.mp4" 0 120; }
r arcane_brown  '{"style":"arcane","skin":"brown"}'       '#14202a' '0.5,0.45,0.45,#3d6470'
r spider_medium '{"style":"spiderverse","skin":"medium"}' '#1c1240' '0.5,0.45,0.42,#e0307a'
r toon_light    '{"style":"toon","skin":"light"}'         '#f2c230' '0.5,0.45,0.45,#ff7a2a'
r clay_tan      '{"style":"clay","skin":"tan"}'           '#cfd6dc' '0.5,0.45,0.45,#f4f1ea'
r real_dark     '{"style":"realistic","skin":"dark"}'     '#1d1f24' '0.5,0.45,0.4,#4a4f5a'
