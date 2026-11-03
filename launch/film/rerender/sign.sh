#!/bin/bash
# sign: showcase/sign-handkit/film.json (film.py: HANDKIT a letter a beat, then I-LOVE-YOU), plus a
# little temporal noise against spotlight banding. Straight copy, 8.8 s.
source "$(dirname "$0")/common.sh"
S="$H/showcase/sign-handkit"; D="$W/sign"; mkdir -p "$D"
render "$S/film.json" "$D/film_raw.mp4" --background "#1c1f26" --spotlight 0.5,0.42,0.42,#323846 render
cut sign "$D/film_raw.mp4" 0 264 "fps=30,noise=alls=3:allf=t"
# The code overlay in act 5 reads the clip's joints: regenerate them with the clip.
python "$S/joints.py"
