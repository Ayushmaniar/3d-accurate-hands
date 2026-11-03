#!/bin/bash
# Shared settings for the launch film's re-render scripts (sourced by each script in this folder).
# Every Handkit-made clip in launch/film/assets/v is rebuilt from its source clip JSON with the
# current binaries, then cut into the asset at the exact frame offset the film was built with.
set -euo pipefail
export PATH="$HOME/tools/ffmpeg-n9.0-latest-win64-gpl-shared-9.0/bin:$PATH"
H="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"          # the Handkit repo root
P="$H/starters/fframes-clip/target/release/fframes-clip.exe"        # landscape player (1920x1080)
HK="$H/target/release/handkit.exe"
A="$H/assets"
V="$H/launch/film/assets/v"                                          # what the film reads
OLD="$H/launch/film/work/old_assets"                                 # previous assets are kept here
W="$H/launch/film/work/rerender"                                     # source renders (intermediate)
mkdir -p "$W" "$OLD"

# Asset encode: H.264 yuv420p, 30 fps, no audio (none of the film's clips has audio), GOP 15 as before.
CRF_DEFAULT=17
enc() { echo "-an -c:v libx264 -preset medium -crf ${1:-$CRF_DEFAULT} -pix_fmt yuv420p -g 15 -movflags +faststart"; }

log() { echo "[$(date +%H:%M:%S)] $*"; }

# render <clip.json> <out.mp4> [player args...]: one player render, timed, log next to the output
render() {
  local clip="$1" out="$2"; shift 2
  local s=$(date +%s)
  log "render $(basename "$(dirname "$clip")")/$(basename "$clip") -> ${out#$H/}"
  "$P" --clip "$clip" --assets "$A" "$@" -o "$out" > "${out%.mp4}.log" 2>&1 || { cat "${out%.mp4}.log"; return 1; }
  log "  done in $(( $(date +%s) - s )) s"
}

# install <name> <new.mp4>: back up the current asset into work/old_assets (never overwriting a
# file there; skipped if an identical copy is already kept), then move the new file into place.
install_asset() {
  local name="$1" new="$2" cur="$V/$1.mp4"
  if [ -f "$cur" ]; then
    if cmp -s "$cur" "$new"; then log "$name: unchanged"; rm -f "$new"; return 0; fi
    local kept=""
    for f in "$OLD/$name.mp4" "$OLD/$name".*.mp4; do if [ -f "$f" ] && cmp -s "$f" "$cur"; then kept="$f"; fi; done
    if [ -z "$kept" ]; then
      local dst="$OLD/$name.mp4" n=2
      while [ -e "$dst" ]; do dst="$OLD/$name.$n.mp4"; n=$((n + 1)); done
      cp "$cur" "$dst"; log "$name: old asset kept as ${dst#$H/}"
    fi
  fi
  mv -f "$new" "$cur"; log "$name: installed"
}

# cut <name> <src.mp4> <first frame> <frames> [-vf chain, default fps=30] [crf]
# Takes <frames> frames starting at source frame <first frame> (30 fps) into assets/v/<name>.mp4.
cut() {
  local name="$1" src="$2" f0="$3" n="$4" vf="${5:-fps=30}" crf="${6:-$CRF_DEFAULT}"
  local ss=()
  # seek half a frame early so the decoder lands on frame f0 exactly (timestamps restart at 0)
  if [ "$f0" -gt 0 ]; then ss=(-ss "$(python -c "print(f'{($f0 - 0.5) / 30:.6f}')")"); fi
  ffmpeg -v error -y ${ss[@]+"${ss[@]}"} -i "$src" -vf "$vf" -frames:v "$n" $(enc "$crf") "$W/_$name.mp4"
  install_asset "$name" "$W/_$name.mp4"
}
