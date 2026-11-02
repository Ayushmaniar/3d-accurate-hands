#!/bin/sh
# Renders the fframes reference cell of each look (medium skin) for compare.mjs:
#   sh tools/looks/ff_cells.sh <pose.json> <outdir> [cell=1080]
# Needs `cargo build --release -p handkit-fframes --example looks` (and FFmpeg's bin on PATH).
set -e
root="$(cd "$(dirname "$0")/../.." && pwd)"
pose="$1"; out="$2"; cell="${3:-1080}"
mkdir -p "$out"
for s in realistic toon spiderverse arcane clay; do
  "$root/target/release/examples/looks" "$pose" "$out/ff_$s.png" "$cell" --only "$s" --tone medium 2>&1 | grep -v '^wrote' || true
done
