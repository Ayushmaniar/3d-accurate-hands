# hands-burrows-roberts bench_144-156: recreation notes

Files: `clip.json` (generated, 39 keys, 30 fps, 12 s, size 1440x1080), `out.mp4` (fframes render 1920x1080),
`out_4x3.mp4` (centre crop), `cmp.sheet.png` / `cmp.side.mp4` (compare.py), `check.txt` (final check).
Authoring sources: `gen.py` (camera, lap plane, finger shapes, helpers) + `keys.py` (poses and timeline);
`python gen.py` rewrites clip.json and re-solves the two-hand contact poses into `poses/*.json`.
`pair.sh` makes reference-over-render pairs at chosen times; `proj.py` maps frame fractions to world points.

Render command (from this folder):
`fframes-clip --clip clip.json --assets ../../../assets --background "#1c1c1c" --spotlight "0.889,0.6,0.62,#5c5c5c" render -o out.mp4`
(the dark background plus a soft grey spotlight stand in for the light apron behind the hands).

Hand naming: the performer faces the camera, so the screen-left hand is Handkit `right` and the screen-right hand is
`left`. Below, "R" = screen-left hand, "L" = screen-right hand; positions are fractions of the 4:3 frame (x, y from top-left).

## What the reference does
- 0.0-0.9 s: both arms hang straight down from the top corners (elbows under rolled sleeves at y~0.15), hands loose
  beside the thighs: R at (0.2, 0.55-0.9) palm inward, fingers curled; L at (0.8, 0.55-0.85), slowly turning its palm
  toward the camera, fingers loosening (0.5-0.75).
- 1.0-1.25: R swings fast (blurred) onto the lap centre-left, flat, fingers spread and pointing down; L curls into a
  loose fist low on the right.
- 1.5: R flicks back out to the left edge, palm forward, fingers spread; L starts swinging across.
- 1.75-2.75: L forearm crosses diagonally from the top right; L lands over R at centre-left and the two hands are
  loosely entwined (x 0.2-0.47, y 0.35-0.8), with a small sway.
- 3.0: both flat on the lap side by side, fingers down (R x 0.2-0.4, L x 0.45-0.6, y 0.35-0.8).
- 3.25-3.5: R flicks out to the left edge (blurred) and L slides left; at 3.75-4.0 R comes back and lies on top of
  L's hand and wrist.
- 4.25: the hands separate on the lap (R left, L right). 4.5: R swings out and up; by 4.75 R is at the belly centre,
  forearm nearly horizontal from the left, palm up, fingers curled (cupped). L moves out to hang at the right side.
- 5.0-6.0: R holds at the belly; L hangs loose at (0.7-0.85, 0.3-0.7), fingers fidgeting, palm turning toward the camera.
- 6.0-6.5: L swings in and settles on top of R: hand over hand at the belly (x 0.3-0.7, y 0.27-0.52), forearms coming
  from both sides, held still until about 8.0.
- 8.0-8.5: R slides out from underneath to the upper left and drops onto the lap, fingers down; L stays near the
  centre, fingers pointing left. 8.75: the hands have slid apart on the lap (R x 0.3-0.5, L x 0.5-0.72).
- 9.0: R flicks up, palm showing (blurred); L lies on the lap right of centre.
- 9.25-9.5: R returns to the belly, back of the hand to the camera, fingers curled down-right; L goes out to hang at the right.
- 10.0-10.75: L hangs at (0.6-0.8, 0.3-0.7), wrist bent, palm forward, fingers spread and curled, small fidgets.
- 11.0-12.0: L swings back over R; the hands are folded at the belly (x 0.35-0.62, y 0.3-0.52) and held to the end.

## How I matched it
- Camera `[0, 0.55, 1.0] -> [0, 0.02, 0.08]`, fov 22: slightly above, looking down about 28 degrees. The elevation
  matters: it makes elbows that sit back near the body project upward, so forearms from the top corners can leave the
  frame even though the wrists are only 0.35-0.55 frame heights down. The scale is set so a hanging hand spans about
  0.35 frame heights, as in the reference.
- Imagined lap: a plane tilted to face the camera (normal [0, 0.45, 0.9]). Lap hands are placed by intersecting the
  camera ray through a target frame fraction with that plane (`on_lap`), palm into the plane, fingers down its slope.
- Every hand is placed with `place {wrist, palm_facing, fingers_toward, forearm_toward}`. `forearm_toward` is computed
  so the elbow lands on, or just outside, a chosen frame edge (`edge_elbow`): the top edge for hanging and lap poses,
  the side edges for the belly poses. Seven finger shapes (relaxed, loose, curled, flat, spread, claw, wrap) are reused.
- Two-hand contacts (L over R at 2.25, both on the lap at 3.0, R on top of L at 3.75, the pair at 4.25, and the two
  hand-over-hand folds) are built as two-hand pose files and run through `handkit solve` with a vary-only goal over all
  fingers plus one hand's position. The solver separated the hands (for example it moved the top hand 2-3 cm) and the
  solved values go into the clip.
- Timing: one key per hand event. Fast swings use move 0.2-0.3 s; approaches use `pass` waypoints lifted toward the
  camera so the top hand comes over the other rather than through it. Before the R slide-out at 8 s, R first opens its
  fingers and L lifts 8 cm; otherwise R's curled fingers pass through L.
- I iterated by pairing reference and render frames at the same times (`pair.sh`). The biggest corrections were
  (1) camera elevation and zoom so the forearms come from the top as in the reference, (2) the lap hands were 0.15-0.2
  frame heights too low, (3) the belly height, and (4) the timing of the R slide-out and of L leaving the lap.

## Final `handkit check`
```
clip: 12.00 s, 360 frames at 30 fps
goals: none held
penetrations: none
speed right: ok (wrist peak 0.59 m/s at 1.10 s)
speed left: ok (wrist peak 1.07 m/s at 1.60 s)
pops: none
framing: every fingertip stays in view
framing: an elbow (where the forearm ends) is in view in 6.10-6.13 s, 8.40 s, 9.20-9.30 s, 10.83-10.87 s; move it out of frame
1 problem(s)
```
Not fixed: an elbow is in view for about 8 frames in total, during fast swings where the forearm direction is
interpolated between two edge-aimed poses. The reference itself has the elbows in frame the whole time (under the
sleeves), and the belly poses need the elbows right on the frame edge, so I aimed every elbow at or just past an edge
and accepted these transients rather than add more waypoints.

## How close it is (honest)
- Good: the overall composition (arms from the top corners, hands in the lower middle, the belly fold at the right
  height), the order of the beats and their timing to within about 0.1-0.2 s, the hand-over-hand settles at 6.5 and
  11.25, the slide-apart at 8.25-8.75, and L hanging and fidgeting on the right.
- Weaker: the reference hands are relaxed and individual (entwined fingers at 2.25, a cupped palm-up R hand half hidden
  at the belly); mine are tidier and more symmetric. The 2.25 entwining reads as one hand lying on the other, not
  interlaced fingers. The blurred flicks (1.0, 1.5, 3.3, 4.5, 9.0) are my guesses at the pose, and the player has no
  motion blur, so they read as crisp snaps. The forearms are thin uniform capsules and look longer than the
  reference's foreshortened, sleeved forearms. The performer leans at 1.25 and 3.75, which shifts the reference
  framing; I ignored that. The lap is a convenience plane, so hands on the lap do not tilt with real thighs.
- Rough fit: hand positions are mostly within about 0.05 of the frame at the sampled times; the worst spots are about
  0.1 off (R at 8.25, L at 9.5).

## Problems, confusions and missing features in Handkit
1. The spec does not say what happens to a hand that a key leaves out. I expected it might be pinned at that key; I
   had to test with `sample`. It is not pinned: each hand interpolates between its own keys, and a move can span keys
   that only name the other hand. The spec should state this.
2. There is no per-hand timeline, and one key cannot give the two hands different `move`/`ease`. I wrote a generator
   that emits one key per hand event; hand-writing 39 keys with full placements would have been very error-prone.
3. Placement is world-space only. To match a video I needed "put the wrist at this point of the frame" and "aim the
   elbow at this edge of the frame", so I wrote my own camera projection (`proj.py`, `on_lap`, `edge_elbow`). A
   screen-space placement (frame fraction plus depth) would remove most of that math.
4. `forearm_toward` is a direction only; the elbow lands 0.26 m away, so "elbow just off the frame" needed a
   ray/sphere intersection by hand. An elbow target point, or an "elbow outside the frame" goal, would help.
5. The check's elbow rule conflicts with references whose elbows are in view: this one shows the elbows (under
   sleeves) the whole time. The rig has no sleeve or fade, so the forearm end is a visible rounded cap. The rule is
   right for the rig, but there is no way to say "the forearm continues out of frame" (for example a longer forearm
   or a fade at the end).
6. `handkit solve` on the "under" pair (two hands overlapping about 2 cm) reported `MISS clear ... -19.9 mm` and left
   8 penetrations. It only cleared them after I moved the top hand 4 cm toward the camera by hand. Starting too deep in
   a collision fails without an error exit (it prints its changes and writes the file), so my generator had to parse
   `query` text to notice.
7. The mid-move "fingers slide past" behaviour produced fingertip pops of 20-150 mm (reported by `check`) whenever one
   hand crossed the other. The fix was always to re-route by hand (lifted waypoints, open the fingers first). A
   "route around" option for the moving hand, or `check` naming the key pair that causes a pop, would save iterations.
8. I parsed `query` text output ("penetrations (overlap > 2 mm): none") to detect failures; fragile. (`query --json`
   exists; I did not check whether `solve` has a machine-readable result.)
9. `sheet` honours the clip's 4:3 `size` while the fframes player renders 16:9. That is fine once known, but the 16:9
   side areas show elbow caps that the 4:3 check does not count.
10. The player has no motion blur; the reference's fastest beats are blurs, so the recreation reads snappier.
11. `compare.py`'s single-row 12-frame sheet at 240 px high is too small to judge hand shapes; I wrote `pair.sh`
    (4 reference/render pairs per image) instead.

## Iterations
About 12 render-and-compare iterations (4 large: camera and scale, lap height, contact poses, timing; the rest small
position and timing tweaks), plus about 10 `check` passes to clear penetrations, speed and pop warnings.
