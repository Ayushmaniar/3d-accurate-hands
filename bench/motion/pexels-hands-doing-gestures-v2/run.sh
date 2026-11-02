#!/bin/bash
# regenerate, check, render, compare grids
export PATH="/c/Users/Rajat/tools/ffmpeg-n9.0-latest-win64-gpl-shared-9.0/bin:$PATH"
python gen.py || exit 1
../../../target/release/handkit.exe check clip.json | tail -n +3
"/c/Users/Rajat/Documents/3D Accurate Hands/starters/fframes-clip/target/release/fframes-clip.exe" --clip clip.json --assets "/c/Users/Rajat/Documents/3D Accurate Hands/assets" --background "${BG:-#5d5c60}" $SPOT render -o out.mp4 2>&1 | tail -1
./cmpgrid.sh g1.png 0 1.5 2.5 3.5; ./cmpgrid.sh g2.png 4.5 5.5 6.5 7.5; ./cmpgrid.sh g3.png 8.5 9.5 10.5 11.5; ./cmpgrid.sh g4.png 12.5 13.5 14.5 15.5; ./cmpgrid.sh g5.png 16 16.5 17 17.8
