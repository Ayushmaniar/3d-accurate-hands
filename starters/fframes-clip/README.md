# fframes-clip

Plays a Handkit clip (`docs/POSE_SPEC.md`, Clips) as an fframes video: an SVG background, an
optional soft spotlight, and the hands rendered in 3D on top. Copy this folder for a video and
restyle `render_frame` in `src/lib.rs`.

```bash
cargo build --release                                   # landscape 1920x1080
CARGO_TARGET_DIR=target-portrait cargo build --release --features portrait   # 1080x1920
```

On Windows, fframes links FFmpeg: set `FFMPEG_DIR` and `LIBCLANG_PATH` (for example in a
git-ignored `.cargo/config.toml`) and put FFmpeg's `bin` on `PATH`.

```bash
fframes-clip --clip clip.json --assets ../../assets strip -n 12       # strip.png
fframes-clip --clip clip.json --assets ../../assets frame 3s          # frames/*.png
fframes-clip --clip clip.json --assets ../../assets render -o out.mp4
```

- `--assets` is a folder with `hand_R.glb` and `hand_L.glb` (`handkit mesh`); without it the
  hands are generated at startup (about a second).
- `--background "#2b2e33"` and `--spotlight 0.3,0.42,0.22,#e9b8b2` (x, y and radius as
  fractions of the frame height, then the color) set the backdrop.
- The video runs at 30 fps; write clips at 30 fps.

fframes is pinned to 1.3.0 with `fframes-media` 1.3.0: fframes 1.4.0 does not build against
`fframes-media` 1.4.1, and `fframes_three_d` must be the version fframes uses so the hands render
into its image type.
