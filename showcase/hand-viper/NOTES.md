Made (pass two, out.mp4): an 8.0 s, 1920x1080, 30 fps Handkit clip (clip.json, 240 frames). Look: realistic, light skin, motion_blur 3 with a 120 degree shutter, dark #262a33 sleeves. Backdrop: #1b1d22 with a soft spotlight (`--spotlight 0.5,0.45,0.42,#3a3f48`). No text, no cuts. Pass one is kept as out_v1.mp4 (work/clip_v1.json, work/compare_steps_v1.png).

The knot is now a real interlaced clasp. Handkit's `interlaced` grip, solved between one person's own right and left hands (work/grip2.py -> gm_sol.json), puts every finger curled over the back of the other hand. The two hands are a mirror pair, with the left hand 1 cm toward the pinkies so the fingers alternate. Each step adds part of that clasp:
- 1.0 s: palms pressed together.
- 2.0 s: ring fingers (and the pinkies with them) cross and hook down over the other hand's back.
- 3.0 s: middle fingers hook over on top.
- 4.0 s: index fingers curl forward over the front, one on the other (the snout).
- 5.0 s: thumbs swing out under the snout, stacked side by side (the lower jaw; right thumb pad on left thumb nail).
- 6.0 s: squeeze: the jaw shuts, with the index tips resting on the thumbs, and the knot is closed.
- 7.0 s: strike: the head rears back (6.6-6.84 s), then lunges right with the index lifted and the thumbs dropped (jaw open). It shuts at 7.3 s and settles until 8.0 s.

Each step moves in over its last 0.2-0.4 s and holds the rest of its second.

Orientation and camera: the knot is seen from the side and slightly in front, looking at the back of the left hand like the reference's profile shots. The head (the thumb/index side) points screen right and a little down. The forearms come in from the bottom left. The camera drifts slowly over 6.6 s.

Pipeline (work/): grip2.py (the clasp) -> steps5.py (the steps, solved in a "knot frame" with touch goals, saved to steps5_sol.json) -> gen2.py (orient.py's orient_profile stands the knot up for the film, and gen2.py writes clip.json) -> cmp2.sh (compare_steps_v2.png: reference frames on top, out.mp4 below, for each step).

Time: pass one about 25 min; pass two about 25 min (08:49-09:14), including a 4 min render.

What got in the way:
- Anatomy against the brief. With palms together and fingers forward, the thumbs are always on top: thumbs underneath a forward-pointing knot would need the fingers to point back at the person. So I kept the reference's own layout: fingers up, head to the side, thumbs below the snout. The cost is the forearms. They come in at about 45 degrees from the bottom left, not horizontal. Laying them flatter needs more radial deviation than the rig's 35 degree wrist limit, with the head pointing straight ahead.
- Two dead ends before the grip: rolling the pass-one knot onto its side (a "clam" with both hands' fingers meeting at the front), and tightening it with plain touch goals. Both left a hole or a tangle of fingers poking out. The interlaced grip between one person's own two hands solved the clasp in one call with no overlap above 1 mm. The first try crossed the left hand at an angle, so I passed the finger joints as `vary` to keep the mirrored placement.
- Thumbs: tip-to-tip goals cross the thumbs (side overlap, as the coordinator explained). Pad-to-pad from a thumbs-up start still crossed them, and the thumbs passed 18-20 mm through each other on the way in. What worked: start both thumbs apart and pointing forward, use pad on nail (stacked), and add a "thumbs apart" pass waypoint at 4.82 s before they close.
- Idle motion (0.2) moves the two hands independently. Inside an interlock that pushed fingers up to 14 mm into each other during holds. Idle at 0.2 is fine now that the keyed holds are clean. The wrist-limit drift from pass one is fixed upstream (PR 17): sampled holds now match the solved poses exactly.
- `check` still lists 11 penetration lines, all inside the knot or behind the near hand:
  - 11 mm between the index fingers for a few frames around 3.83 s, as they curl past each other (the far one is hidden behind the near one).
  - About 7 mm between the index fingers in the step 4 pose itself (far index under near).
  - 2-4 mm between the ring and middle fingers inside the clasp, and between the thumb bases.
  - No pops. Framing is clean.
- With motion_blur 3, fast moves show three distinct copies, not a smear. A 120 degree shutter tightens them, but the strike frames (6.9 s, 7.3 s) still show stepped ghosts.
