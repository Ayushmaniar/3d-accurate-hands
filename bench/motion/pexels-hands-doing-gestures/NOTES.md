# pexels-hands-doing-gestures / seg01: Handkit motion recreation

Deliverables: `clip.json` (30 fps, 18.2 s, 37 keys), `out.mp4` (fframes landscape player,
1920x1080, rendered with `--background "#4a474a" --spotlight "0.78,0.42,0.95,#646265"`).
`clip.json` is generated: `keys.py` holds the poses I read off the reference and `gen.py`
turns them into the clip. Helpers: `cmp3.sh` (reference | render | 50 % overlay at chosen
times), `sbs.sh` (reference | out.mp4 grid), `zoom.sh` (reference crops), `traj.py`
(speed trace of one site over time). `final_compare.png` is reference vs. final render at
12 times; `strip.png` is the player's strip; `ref/sheet_*.png` are reference frames every 0.5 s.

## What the reference does

Static camera, grey backdrop with a vignette. The arms are crossed at the wrists: the
**left** hand is on screen left and its forearm comes up from bottom centre-right, always in
front; the **right** hand is on screen right with its forearm from bottom left, behind.
(Palm to camera with the thumb on screen left at 15-17 s fixes which hand is which.) The
motion is slow and continuous, no holds.

| time | left hand (screen left) | right hand (screen right) |
|---|---|---|
| 0-1 s | Hand upright, palm toward camera-left, thumb up in front; index, middle curled over toward the camera, the rest nearly straight. Almost still. | Back of the hand up, index straight pointing right, middle to pinky drooping, thumb hanging below. |
| 1-2 s | Turns edge-on, then the back of the hand rotates to the camera while the hand tips over to point up-left, fingers opening into a V. | Fingers extend and point right; hand drifts right. |
| 2-3 s | Hand lowers to horizontal pointing left, palm down, wrist arched; fingers start to droop. | Palm down, slides right and down, fingers droop. |
| 3-4 s | Wrist rises, the hand stays palm down and arches higher; fingers hang down-left. | Mirrors: wrist rises, fingers hang down-right. |
| 4-5.5 s | Both wrists lift so the hands stand up from the wrist with the fingers folded over sideways at the knuckles (left hand's fingers to the left). | Same, fingers to the right. A symmetrical "swan" pair. |
| 5.5-7.5 s | Wrist bends hard back, the hand leans right and the fingers curl up and over to the left, fanned. | Hand lowers toward bottom right, curled fingers up-right. |
| 8-8.5 s | Hand reaches up tall, fingers fanning up (tips at the top edge). | Hand down at bottom right, flat, palm down, fingers pointing right. |
| 9-11 s | Forearm tilts further left; hand points up-left then left, back of the hand to the camera, index straightest, the others more curled (graded fan). | Low and nearly horizontal, index pointing right, others drooping, fingertips at the bottom edge. |
| 11-12.5 s | Hand lowers to point left, fingers droop. | Rises over the left wrist, index straight pointing right, then up-right, others hooked down. |
| 12.5-14 s | Palm down, fingers droop toward the camera. | Keeps rising; palm turns toward the camera, index pointing up-right. |
| 14-15.5 s | Rises and turns palm to the camera; fingers curl over. | Upright, palm to the camera, fingers curling up-right. Both hands now upright in a V. |
| 15.5-18.2 s | Upright, palm to camera, thumb up on the left, index straight, other fingers curled; drifts slowly left. | Lowers and tilts right, fingers flowing to the right. |

## How I matched it

1. Extracted frames every 0.5 s (and crops) and wrote, per hand per key: wrist and middle
   knuckle positions in screen pixels, the point where the forearm leaves the frame, the palm
   direction, and finger flexion (`keys.py`).
2. `gen.py` maps screen pixels to the plane z = 0 (0.265 mm per 1920-px pixel, camera at
   z 0.535 m, fov 30) and writes `place: {wrist, palm_facing, fingers_toward}`. Because
   `place` fixes the hand and the wrist angles swing the forearm, it solves `wrist_flex` and
   `wrist_dev` so the forearm points where it does in the video, searching the forearm's tilt
   in depth to stay inside the wrist limits (the dancer's wrists bend further than the rig
   allows at 6-8 s and 17.5 s, so those are clamped and the forearm is a little off there).
   The left wrist sits 3.5 cm in front of the right so the left forearm crosses in front.
3. Timing: one key every 0.5 s, `ease: smooth`, `move` = 1.7 x the gap so moves overlap
   and the motion never stops. Overlapping moves run ahead of the keys by (move - gap)/2
   (measured), so every key's `t` is shifted 0.175 s later to keep the poses on time.
4. Compared reference, render and overlay at about 40 times (on and between keys), fixed
   palms, finger curls, thumb positions and placement, and repeated.
5. Backdrop sampled from the reference: darker corners (#4a474a), lighter centre left via the
   spotlight.

## Final `handkit check clip.json`

```
clip: 18.20 s, 547 frames at 30 fps
goals: none held
penetrations: none
speed right: ok (wrist peak 0.09 m/s at 6.27 s)
speed left: ok (wrist peak 0.10 m/s at 2.27 s)
pops: none
framing: a fingertip leaves the camera's view in 0.00-0.53 s, 1.77-2.03 s, 8.80-10.20 s
1 problem(s)
```

I did not fix the framing report. In the reference the fingertips also graze the frame at
these moments: the top edge at 0-0.5 s, the left edge at about 2 s, and the right hand's
drooping fingertips at the bottom edge at 9-10 s. The rig's fingers are slightly long compared
with the dancer's, so its tips cross the edge by a few pixels. Pulling the hands in would
move the composition away from the reference. I already lowered the left hand where that
cost nothing (0, 0.5, 2, 6.5, 8-10, 16 s), which cleared 6.4-6.8, 8.1-8.7 and 16.0-16.3 s.
Earlier issues it reported that I did fix: forearms passing through each other (5.8-7.7 s,
10.3-10.7 s, 15-18 s; fixed with depth offsets and forearm tilt), and a thumb pop at 4.63 s.
That pop came from the thumb sliding past the curling index; I moved the thumb out of the
fingers' path.

## How close it is (honest)

- **Composition and timing: close.** The crossed forearms, where each hand is, which way it
  points, and how high it is track the reference second by second. Overlays at 40 times
  usually land the wrists within about 10-20 px of 480. The big phrases happen on time:
  lower, arch, swan pair, the left hand reaching up, the right hand rising over the wrist,
  both palms to the camera.
- **Palm facing: mostly right.** It is approximate at 1.5-2 s and 12.5-14 s (left hand) and
  13-14 s (right hand), where the reference turns quickly.
- **Fingers: the weakest part.** The dancer's fingers are long, slender and finely graded,
  with small waves inside each 0.5 s. Mine are graded per key and look stiffer: often more
  hooked, sometimes curling toward the camera where hers curl across the frame. The
  in-between finger flow is gone because of the 0.5 s keys and the smoothing.
- **Hand shape:** the rig hand is broader and blockier, and it creases sharply at the wrist
  when bent far back (6-8 s). The reference wrist curves smoothly.
- **Smoothing:** the overlap blend rounds off quick turns. The left hand rising at 14-14.5 s
  and the turn at 1.5-2 s arrive a little late and a little less far.
- Skin and lighting differ (ignored as instructed).

## Problems, confusions and missing features in Handkit

1. **No way to move continuously through keys.** With `natural` or `smooth`, every key is a
   full stop. With keys every 0.5 s, `traj.py` showed the speed falling to 0 at every key, so
   the hand would pulse twice a second. `linear` avoids the stops but changes velocity
   sharply at each key. My workaround was overlapping moves (`move` = 1.7-2 x the gap), which
   is smooth. But **overlapping moves run ahead of the keyed path by about (move - gap)/2**.
   That is not documented; I measured it on a test ramp (move 1.0, gap 0.5: 0.25 s early) and
   shifted every key to compensate. I expected a spline or "flow through" ease, or at least
   overlap that stays centred on `t`.
2. **No way to say where the forearm points while placing the hand.** `place {wrist,
   palm_facing, fingers_toward}` fixes the hand, so the forearm direction falls out of
   `wrist_flex` and `wrist_dev`. In this video the forearm angle carries much of the
   composition. I had to write my own inverse (`gen.py: wrist_angles` plus a search over the
   forearm's depth tilt to stay inside the wrist limits). I expected something like
   `forearm_toward` or `elbow` alongside the hand frame, with handkit solving the wrist
   angles, or at least a report of the resulting forearm direction.
3. **Wrist range is narrower than this performer's.** She reaches about -80 to -88 deg of
   extension with 35 deg radial deviation (6-8 s) and more than -25 deg ulnar deviation
   (15-18 s). The rig clamps at -70 / 35 / -25, so the forearm is off at those moments. I
   clamped in my generator; nothing in handkit told me the requested shape was out of range,
   because with `place` the forearm simply goes wherever the clamped angles put it.
4. **`check` framing gives no detail.** It does not say which fingertip, which edge, or by how
   much. I had to render frames to learn that the reports were the left, top and bottom edges
   by a few pixels, and that the reference does the same.
5. **`check` pops give no cause.** "right.thumb.tip jumps 23.0 mm at 4.63 s" was the
   anti-penetration slide (thumb against the curling index), but the message doesn't say so.
   I guessed and moved the thumb.
6. **Forearm-forearm penetration is reported with times but not where along the forearms.**
   There is no way to say "left forearm in front of right" for a clip. I fixed it with
   per-key depth offsets and tilts, found by trial (3 rounds).
7. **`sample` writes placement as `elbow` + `forearm_rotation_deg`, not as the
   wrist/palm/fingers I wrote.** To read the hand position over time I had to run `fk` on
   every sample. `fk` rounds positions to 1 mm, which makes speed traces at 30 fps noisy.
8. **No reference comparison.** `sheet` and the player's `strip` show only the clip. Most of
   my effort went into building ffmpeg side-by-side and overlay sheets against the video. A
   `sheet --ref video.mp4 [--overlay]` would have saved many iterations.
9. **Mesh look.** The hand is broad, thick through the metacarpals and slightly long in the
   fingers relative to this slender hand, and there is no proportion or size control. At
   large wrist extension the skin creases into a hard kink. An edge-on hand still reads as a
   wide block, which makes "is the palm facing left or the camera" hard to judge in renders.
10. Minor: thinking about which rig hand is which needed care, because the arms are crossed;
   the spec's mirror and thumb-side rules were enough once applied. `fingers_toward` is the
   metacarpal direction, so a hand whose fingers curl reads as pointing elsewhere; it is
   documented, but easy to forget when reading poses off video.

What worked well: `place` with free, non-orthogonal vectors; `from`-less keys inheriting the
previous pose; `check` catching forearm overlaps and the thumb pop; the player's `frame` with
comma-separated times and `--scale` for fast comparison renders (about 1 s per batch); the
spotlight option for matching the vignette; a full render in about 22 s.

## Iterations

About 14 render-and-compare rounds: one calibration, 6 per-segment pose passes, 2 timing and
smoothness experiments, 3 penetration and framing fixes, and 2 whole-clip review passes. That
is roughly 45 `cmp3.sh` / `sbs.sh` comparison images.
