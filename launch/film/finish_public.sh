#!/bin/bash
# mux the rendered picture with the mix, make the phone copy, verify
set -e
export PATH="$HOME/tools/ffmpeg-n9.0-latest-win64-gpl-shared-9.0/bin:$PATH"
F="$(git rev-parse --show-toplevel)/launch/film"; HK="$(git rev-parse --show-toplevel)"
SRC="${1:-$F/out/render_public.mp4}"
cd "$F"
# The music is mono (MusicGen): widen the mix a little, then bring it back to about -14.5 LUFS
# with the peak under -2 dBTP (earshot critique: stereo width in range).
mkdir -p work && ffmpeg -v error -y -i assets/audio/mix.wav -af "stereowiden=delay=18:feedback=0.25:crossfeed=0.25:drymix=0.85,volume=4.1dB,alimiter=limit=0.74:level=false:attack=3:release=60" -ar 48000 work/mix_wide.wav
ffmpeg -v error -y -i "$SRC" -i work/mix_wide.wav -map 0:v:0 -map 1:a:0 -c:v copy -c:a aac -b:a 256k -cutoff 18000 -ar 48000 -t 73.8 -movflags +faststart out/handkit-launch-public.mp4
ffmpeg -v error -y -i out/handkit-launch-public.mp4 -i work/mix_wide.wav -map 0:v:0 -map 1:a:0 -t 73.8 -vf "scale=960:540:flags=lanczos" -c:v libx264 -preset slow -crf 28 -pix_fmt yuv420p -c:a aac -b:a 160k -cutoff 17000 -movflags +faststart "$HK/phone/handkit-launch-public.mp4"
for f in out/handkit-launch-public.mp4 "$HK/phone/handkit-launch-public.mp4"; do
  echo "== $f"; ffprobe -v error -show_entries format=duration,size -show_entries stream=codec_name,width,height,r_frame_rate,sample_rate,channels -of compact "$f"
  ffmpeg -hide_banner -nostats -i "$f" -map 0:a -af ebur128=peak=true -f null - 2>&1 | sed -n "/Summary/,\$p" | grep -E "I:|Peak:" | sed -n "1p;3p"
done
# contact sheet of the final: 30 frames, one every 2.46 s, timestamped
ffmpeg -v error -y -i out/handkit-launch-public.mp4 -vf "select='isnan(prev_selected_t)+gte(t-prev_selected_t\,2.45)',scale=384:-2,drawtext=fontfile='C\:/Windows/Fonts/arial.ttf':text='%{pts\:hms}':x=6:y=6:fontsize=16:fontcolor=white:box=1:boxcolor=black@0.6,tile=6x5:padding=4:color=black" -fps_mode vfr -frames:v 1 out/contact-sheet-public.png
# key sync frames
for t in 10.967 19.1 27.1 50.3 51.867 60.833 70.833; do ffmpeg -v error -y -ss $t -i out/handkit-launch-public.mp4 -frames:v 1 -vf scale=480:-2 work/keyp_$t.png; done
ffmpeg -v error -y $(for t in 10.967 19.1 27.1 50.3 51.867 60.833 70.833; do echo -i work/keyp_$t.png; done) -filter_complex "hstack=7" out/key-frames-public.png
echo done
