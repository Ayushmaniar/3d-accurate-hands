---
name: handkit
description: Animate anatomically accurate 3D hands and forearms for code-made video (fframes, HyperFrames/three.js, Remotion). Use when a video needs hands that gesture, sign, count, point, hold hands, shake hands, react, or recreate hand choreography from a reference clip; in realistic skin or toon, Spider-Verse, Arcane or clay looks, any skin tone, any number of people.
---

# Handkit

Hands are JSON: a **pose** is joint angles plus a placement; a **clip** is poses keyed over
time; the core turns them into natural motion (springs, finger cascade, follow-through, idle)
and an engine draws them. Everything is deterministic: the same clip gives the same frames.

`HANDKIT` below is the Handkit folder (the repo root). Run commands from it.

```bash
H=$HANDKIT/target/release/handkit          # the CLI (cargo build --release once)
P=$HANDKIT/starters/fframes-clip/target/release/fframes-clip   # the video player
A=$HANDKIT/assets                          # hand_R.glb, hand_L.glb (handkit mesh writes them)
```

## The loop (do this, in order)

1. **Start from the library.** `$H gestures` lists ~50 named poses with descriptions (fist,
   point, ok, thumbs_up, peace, spread_palm, pinch, heart_hands, camera_frame, handshake...).
   `"from": "ok"` on a hand takes a gesture's shape; that is nearly free. Write joint
   angles only for what differs.
2. **Write a clip** (minimal, 30 fps):
   ```json
   { "version": 1, "fps": 30,
     "camera": { "position": [0, 0.12, 0.6], "target": [0, 0.1, 0], "fov": 30 },
     "look": { "style": "realistic", "skin": "medium" },
     "keys": [
       { "t": 0,   "hands": { "right": { "from": "fist",
           "place": { "wrist": [0, 0, 0], "palm_facing": [0, 0, 1], "fingers_toward": [0, 1, 0] } } } },
       { "t": 0.8, "hands": { "right": { "from": "spread_palm" } } },
       { "t": 1.6, "ease": "snappy", "hands": { "right": { "from": "point" } } } ] }
   ```
   Each key starts from the previous one: write only what changes. `t` is when the pose
   arrives; `move` is how long the move takes (0.12 = a flick); `"pass": true` flows through
   a key without stopping. Place hands where they appear in the frame with
   `"place": {"screen": [x, y], "depth": 0.5, "palm_facing": .., "fingers_toward": ..}`
   (x, y are fractions of the frame, 0,0 top-left) and bring the arm in from an edge with
   `"elbow_screen": [1.1, 0.9]`.
3. **Check** before rendering: `$H check clip.json` (held contacts, fingers through fingers,
   speeds, pops, framing, elbows in view). Fix what it says; it names the keys.
4. **Look** small and cheap: `$P --clip clip.json --assets $A strip -n 12` writes strip.png
   (read it). `frame 2.5s` writes one full frame.
5. **Render**: `$P --clip clip.json --assets $A render -o out.mp4` (1920x1080, seconds).
   Portrait 1080x1920: the same commands with
   `$HANDKIT/starters/fframes-clip/target-portrait/release/fframes-clip`.
   Backdrop: `--background "#1d1f24"`, `--spotlight 0.5,0.45,0.35,#3a3f48`.

## What else you can say

- **Contacts**: `"goals": [{"touch": ["right.thumb.tip", "right.index.tip"]}]`, then
  `$H solve pose.json` meets them exactly (exit 2 if it can't). Aim: `{"aim": "right.index", "at": [x,y,z]}`.
- **Looks**: top-level `"look"`: style `realistic | toon | spiderverse | arcane | clay`, skin
  `light | medium | tan | brown | dark | #rrggbb`, `outline`, `motion_blur` (sub-frames).
  Or on the player: `--look spiderverse,skin=brown`.
- **The forearm's end**: the rig stops at the elbow. When an elbow would show, use
  `"look": {"arm": "fade"}` or `{"arm": {"sleeve": "#1e2a4a", "length": 0.08}}` (a rolled sleeve).
- **Sync a UI to the hand**: `$H track clip.json --site right.index.tip` prints where a site is
  on screen every frame (frame fractions), e.g. to make a button react to a tap.
- **People**: hands `ana.right`, `ben.left` (any names); `"people": {"ana": {"look": {"skin": "dark"}, "scale": 1}}` (scale 0.75 is a child's hand).
  Hands push each other aside instead of passing through (`"collide": false` to allow overlap).
- **Holding hands**: a goal `{"grip": ["ana.right", "ben.right"], "style": "handshake"}`;
  styles handshake, interlaced, cupped, high_five, fist_bump, pinky. Keys in a row with the same
  grip move the pair together, the follower trailing a few frames.
- **Two hands of one person** clasp, clap or frame a shot with `touch` goals between them.

## Recreating a reference video

Study it first: `ffmpeg -i ref.mp4 -vf fps=4,scale=480:-1 f%03d.png` and read the frames. One
key per visible event; place hands with `screen`, aim forearms with `elbow_screen`; time keys
to the frames. Compare: `python $HANDKIT/tools/motion/compare.py ref.mp4 out.mp4 cmp` (a
sheet of both at the same times, and a side-by-side MP4).

## When you need more

`$H explain` (conventions), `$H explain clips`, `$H explain looks`, `$H explain <param>`
(e.g. `thumb.cmc_rot`), or `$HANDKIT/docs/POSE_SPEC.md`. Read only the part you need.
Raw joint ranges: fingers `mcp_flex` -30..95, `pip_flex` -5..110, `dip_flex` -10..85,
`mcp_abd` about -15..25; wrist `wrist_flex` -80..85, `wrist_dev` -35..35.

Rules of thumb: library first; few keys, good timing (real hands move in 0.2-0.6 s and hold);
check before render; look at a strip, not the whole video; keep files small.
