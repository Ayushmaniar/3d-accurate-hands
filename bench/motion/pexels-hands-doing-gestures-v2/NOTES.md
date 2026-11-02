# pexels-hands-doing-gestures seg01: recreation notes

Files: `clip.json` (the clip, generated), `out.mp4` (fframes render, 1920x1080, 30 fps, 18.2 s),
`keys.py` (the key table: wrist and knuckle positions read off the reference in 480x270 pixels,
palm direction, hand shape per hand per 0.5 s), `gen.py` (turns the table into `clip.json`: pixel
to world mapping, named finger shapes), `run.sh` (generate, check, render, compare grids),
`cmpgrid.sh`, `g1..g5.png` / `gall.png` (reference over ours at the same times), `cmp.sheet.png`
and `cmp.side.mp4` (from `tools/motion/compare.py`), `ref/` (reference frames at 2 fps).

Render command: `fframes-clip --clip clip.json --assets ../../../assets --background "#4e4c4e"
--spotlight 0.889,0.42,0.75,#646266 render -o out.mp4`.

## What the reference does

Static camera, grey backdrop with a soft lighter centre. The left forearm rises from bottom
centre-right toward the upper left; the right forearm comes from bottom centre-left toward the
right; they cross in an X in the lower middle the whole time. Slow continuous motion, no holds.

- 0-1 s: left hand high on the left, wrist bent back, fingers up, palm toward the right/viewer,
  index tallest. Right hand lower right, palm down, index reaching right, other fingers curled.
- 1-2 s: left hand tips over to the left and the fingers fan open; right hand drifts down right.
- 2-3 s: left hand lies out flat to the left, palm down, fingers long and level (2.5 s); then the
  wrist rises and the fingers droop (3 s). Right hand low right, palm down, fingers drooping.
- 3.5-4.5 s: both wrists rise and arch, fingers hang down from the knuckles: two "swan necks"
  facing each other above the crossing.
- 5-6 s: left hand turns so the wrist is a sharp bend with the fingers curled over toward the
  right (swan head); right hand rises beside it, also bent, fingers pointing up/right.
- 6.5-7.5 s: left hand climbs, wrist bent hard, fingers curving up and back; right hand sinks low
  right, fingers pointing right.
- 8-9 s: left hand high at upper left, fingers fanned and curved up (claw-like flourish);
  right hand low right, palm down, index reaching right.
- 9.5-11.5 s: left hand turns palm-down and sweeps right and down with fingers trailing; the right
  hand rises from bottom right to the crossing, fingers drooping/pointing right.
- 12-13.5 s: both hands meet low in the middle, palms down, knuckles bent, fingers dangling;
  right hand then lifts, fingers pointing up right.
- 14-15 s: both hands turn palms toward the viewer and rise; ballet hands: index tall, middle
  and ring dipping toward the thumb.
- 15.5-18.2 s: left hand holds the ballet hand upright at left centre; right hand opens and
  tips over to the right, fingers pointing right, palm down-ish.

## How I matched it

- Camera fixed at z = 0.5 m, fov 35, looking at the origin; a pixel-to-meter mapping (frame
  height = 0.31 m at z = 0) so wrist and knuckle positions could be read straight off reference
  frames at 480x270.
- 35 keys, one every 0.5 s, each with both hands. For each: wrist pixel, knuckle pixel (gives
  `fingers_toward` in screen plane), a hand-chosen `palm_facing`, a fixed `forearm_toward` per
  hand (left arm toward bottom right, right arm toward bottom left, so the arms cross like the
  reference), small depth offsets (left in front) so the forearms never intersect, and one of
  eight named finger shapes (ballet, ballerina, fan, droop, hook, hooklite, swan, reach, curl).
- All middle keys are `pass: true` so the motion flows like the reference (it never stops).
- `idle` 0.6 for a little life.
- Background `#4e4c4e` plus a large soft spotlight `#646266` to match the brighter centre.
- Iterated by rendering the full video (about 22 s) and building reference-over-ours grids at
  0.25-0.5 s spacing; fixed palm directions, shapes, thumbs, and framing.

## Final `handkit check clip.json`

```
clip: 18.20 s, 546 frames at 30 fps
goals: none held
penetrations: none
speed right: ok (wrist peak 0.10 m/s at 10.33 s)
speed left: ok (wrist peak 0.09 m/s at 2.23 s)
pops: none
framing: every fingertip stays in view
ok
```

Earlier versions reported forearm-forearm penetration (fixed with depth offsets of +-4.5 cm),
fingertips past the top edge (fixed by lowering the left hand at 0-1 s and 6.5-8.5 s), and an
elbow in view at 11.5-14 s (fixed by steepening the right `forearm_toward`).

## How close it is

Good: composition (two forearms crossing in an X from the bottom, hands in the upper two
thirds), where each hand is at every half second, the overall rise/fall rhythm, the timing of
the big beats (flat reach at 2.5 s, double swan neck 3.5-4.5 s, left hand climbing 6-8 s, sweep
down 9.5-11.5 s, meeting low 12-13 s, ballet hands from 14.5 s, right hand tipping over at 16.5 s).

Differs:
- Finger shapes are coarser. The reference fingers are long and continuously varied (each
  finger at its own curl, waves running through them); mine switch between eight preset shapes,
  so the "flowing wave" through the fingers is mostly lost. The ballerina hand (15.5-18 s) is
  the right idea but the middle/ring curl reads as a clump rather than the airy reference shape.
- Wrist bends: the reference wrist bends are deeper and more sinuous (8.5 s, 5.5 s). Mine are
  close in direction but stiffer.
- The rig's hands look shorter and boxier than the reference's long slender fingers, so even at
  matching joint angles the silhouette differs (ignored per brief, but it affects read).
- Some palm orientations are guesses where the reference is ambiguous (10-11 s right hand,
  13.5-14 s right hand turning).
- Depth: reference hands move toward/away from camera; mine stay near fixed depths.

Overall I'd call it a recognisable recreation of the choreography and framing, with hand
poses roughly right per half second but less fluid and less elegant in the fingers.

## Problems, confusions and missing features in Handkit

1. `handkit sheet clip.json --at 0,3 --mesh` and `handkit render`: the image was useful, but the
   sheet's camera/framing is not the fframes player's 16:9 frame at 1920x1080, so I could not use
   it to judge framing against the reference; I went straight to fframes renders (22 s each) and
   my own ffmpeg grids. A `sheet --like-player` (same aspect, background) or a built-in
   "reference row over clip row at the same times" option would save a step; `compare.py` does
   this for the video but needs out.mp4 first.
2. Placing a hand from what you see in 2D is the core task of matching a video, and there is no
   screen-space placement. I wrote my own pixel-to-world mapping (`gen.py`) assuming a pinhole
   camera at z = 0.5. A placement like `{ "screen_wrist": [x, y], "screen_knuckles": [x, y],
   "depth": z }` (fractions of the frame) would make reference matching much faster.
3. `palm_facing` is the hardest value to get right by eye and there is no quick way to try a few:
   I made one-off pose files and rendered them (`render --views hero`) to pick. A `render` that
   takes several palm candidates, or `sample --wrist` style output being directly reusable as a
   key, would help. `sample --wrist` itself worked well for reading back what the clip produced.
4. `forearm_toward` combined with large finger direction changes gives big wrist angles; `check`
   never warned me, and `query` on a sampled pose showed wrist_flex -65 deg with no limit warning
   (inside limits). Fine, but I could not tell whether the rig was silently clamping when my
   requested direction was beyond the wrist; the spec says `query` reports violations, `check`
   does not mention wrist limits at all. A check line "wrist at limit in a-b s" would help.
5. Every key must repeat the whole hand to change placement ("A placement written replaces the
   previous one"), which is fine, but because finger shapes are not interpolated by name I had to
   generate everything from a script. There is no way to blend two gestures (e.g. 60% droop, 40%
   hook) or to run a wave through the fingers (`cascade` only offsets fingers within one move).
   A finger "wave" or per-finger phase offset across `pass` keys would directly serve this kind
   of dance-like reference.
6. The `--spotlight` parameter units (fractions of frame height for x as well as y) are easy to
   misread: x = 0.889 is the horizontal centre of a 16:9 frame. Worth stating in the help.
7. `fframes-clip --help` prints nothing at all (exit with no output), so the flags and the
   `--spotlight` ordering are only in the README.
8. Penetration report for the forearms (`right.forearm x left.forearm`) was clear and fixing it
   via depth was easy; good.
9. The ffmpeg drawtext in my own scripts printed fontconfig warnings; not a Handkit issue.

## Iterations

About 13 render-and-compare iterations of the full clip (each: regenerate, `handkit check`,
fframes render, comparison grids), plus 3 small pose-only experiments (palm direction and
ballet-hand variants via `handkit render`).
