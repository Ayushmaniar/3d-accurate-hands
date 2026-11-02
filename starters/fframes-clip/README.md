# fframes-clip

Plays a Handkit clip (`docs/POSE_SPEC.md`, Clips) as an fframes video: an SVG background, an
optional soft spotlight, and the hands rendered in 3D on top. Copy this folder for a video and
restyle `render_frame` in `src/lib.rs`.

```bash
cargo build --release                                   # landscape 1920x1080
CARGO_TARGET_DIR=target-portrait cargo build --release --features portrait   # 1080x1920
```

On Windows, fframes links FFmpeg: set `FFMPEG_DIR` and `LIBCLANG_PATH` (for example in a
git-ignored `.cargo/config.toml`) and put FFmpeg's `bin` on `PATH` for the build. `build.rs`
copies FFmpeg's DLLs next to the binary, so it runs from any shell; without them Windows stops
it before it prints anything (even `--help`).

```bash
fframes-clip --clip clip.json --assets ../../assets strip -n 12       # strip.png
fframes-clip --clip clip.json --assets ../../assets frame 3s          # frames/*.png
fframes-clip --clip clip.json --assets ../../assets render -o out.mp4
```

- `--assets` is a folder with `hand_R.glb` and `hand_L.glb` (`handkit mesh`); without it the
  hands are generated at startup (about a second).
- `--background "#2b2e33"` and `--spotlight 0.5,0.42,0.22,#e9b8b2` set the backdrop: the
  spotlight is x as a fraction of the frame width (0 left, 1 right), y and radius as fractions
  of its height (0 top), then the color. `handkit sheet clip.json --background "#2b2e33"`
  draws a contact sheet on the same color.
- Looks (`handkit explain looks`): the clip's `"look"`, or `--look toon,skin=tan,outline=4`,
  `--look-json '{"style": "clay"}'`, and `--motion-blur 4` (sub-frames averaged across a 180
  degree shutter). Styles: realistic, toon, spiderverse, arcane, clay; skin light, medium, tan,
  brown, dark or `#rrggbb`. The looks demo cycles through looks, each on its own backdrop with
  its style and skin tone: `fframes-clip --clip ../../clips/demo_looks.json --looks
  "realistic;toon;spiderverse;arcane;clay" --look-every 2 render -o looks.mp4`.
  A 1080p frame takes about 80 ms (stylized) to 110 ms (realistic) on the CPU (16 threads),
  times N with motion blur.
- Any number of hands: a clip with `ana.right`, `ben.left` (`handkit explain`, "Hands and
  people") draws one mesh per hand, each in its person's skin and nails (`"people": {"ana":
  {"look": {"skin": "brown"}}}`; every style shows it, the realistic skin maps toned per
  person). `clips/demo_four_hands.json` is two people stacking four hands.
- `fframes-clip --help` lists these flags and the standard fframes commands.
- The video runs at 30 fps; write clips at 30 fps.

fframes is pinned to 1.3.0 with `fframes-media` 1.3.0: fframes 1.4.0 does not build against
`fframes-media` 1.4.1, and `fframes_three_d` must be the version fframes uses so the hands render
into its image type.
