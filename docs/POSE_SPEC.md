# Handkit pose spec (version 1)

You pose one or two hands, each with a forearm, by writing a JSON pose file of joint angles.
Angles are degrees, positions meters; anything left out is 0. Unknown keys are errors.

## Workflow

1. Start from a library gesture when one is close: `handkit gestures` lists them,
   `handkit gesture ok > pose.json` copies one (or write `"from": "ok"` in your file).
2. Write the joint values for the rough shape, and `goals` for what must be exactly true:
   which surfaces touch, where a finger points.
3. `handkit solve pose.json`: adjusts the values until the goals hold, without fingers passing
   through each other, and writes them back. It prints each goal's status and what changed, and
   exits with code 2 (still writing the file) when a goal is missed.
4. `handkit query pose.json`: limit violations, fingers passing through each other, contacts,
   the nearest part to each thumb tip, goals, `checks` gaps, aim at `probes`, and fingertip
   positions. Text, or `--json` for scripts; no image.
5. `handkit render pose.json --views front,hero --size 384`: a small multi-view PNG to judge
   the look. Views: front, back, side, top, three_quarter, hero (your `camera`).
6. `handkit fk pose.json`: every joint's position and axes and every site, when you need exact
   geometry. `handkit explain thumb.cmc_rot` explains one parameter.

## Format

```json
{
  "version": 1,
  "hands": {
    "right": {
      "place": { "wrist": [0.08, 0, 0], "palm_facing": [0, 0, 1], "fingers_toward": [0, 1, 0] },
      "twist": 0, "wrist_flex": 0, "wrist_dev": 0, "palm_cup": 0,
      "thumb":  { "cmc_flex": 0, "cmc_abd": 0, "cmc_rot": 0, "mcp_flex": 0, "ip_flex": 0 },
      "index":  { "mcp_flex": 0, "mcp_abd": 0, "pip_flex": 0, "dip_flex": 0 },
      "middle": { "mcp_flex": 0, "mcp_abd": 0, "pip_flex": 0 },
      "ring":   { "mcp_flex": 0, "mcp_abd": 0, "pip_flex": 0 },
      "pinky":  { "mcp_flex": 0, "mcp_abd": 0, "pip_flex": 0 }
    },
    "left": { "mirror": "right" }
  },
  "goals": [{ "touch": ["right.thumb.pad", "right.index.tip"] }],
  "checks": [["right.thumb.pad", "right.index.pad"]],
  "probes": { "target": [0.0, 0.3, -0.5] },
  "camera": { "position": [0, 0.05, 0.6], "target": [0, 0.05, 0], "fov": 35 }
}
```

`goals`, `checks`, `probes`, `camera` and `size` (the frame's [width, height], for placing hands
with `place.screen`) are optional; `solve` needs goals. Leave out a hand to show only one. Goals (touch, aim, vary) are explained under "Goals" below.

## Clips

Motion is a clip: keyed poses over time. The text below is the output of `handkit explain clips`.

```text
Clips: motion as keyed poses. A clip file:
  { "version": 1, "fps": 30, "duration": 9.0, "size": [1920, 1080],
    "camera": { "position": [..], "target": [..], "fov": 35 },
    "keys": [ { "t": 0, "hands": {..}, "goals": [..] }, { "t": 1.2, "move": 0.4, ... }, ... ] }
duration defaults to the last key + 0.5 s; size (the video's, for the framing check, `sheet`
and screen placements) to 16:9. A key's place.screen uses the clip's camera as of that key.

A key is a pose file plus timing. Every key starts from the previous key: write only what
changes (a key { "t": 2, "hands": { "right": { "index": { "mcp_flex": 0 } } } } straightens one
finger and keeps everything else). On a hand, "from": "<gesture>" takes that gesture's joint
values (keeping the hand where it is); "from" at the top of a key takes the whole gesture,
placement and goals included. A placement written replaces the previous one. Every hand must be
in the first key. Goals belong to their key only (repeat a goal in the next key to hold it).

Timing, per key (all optional except t):
  t          when the hand arrives at this key's pose (seconds).
  move       how long the move into it takes (default: the whole gap since the hand's last key).
             The hand holds the previous pose until t - move. Short = fast: 0.12 is a flick.
  ease       natural (default: smooth start and stop, a little follow-through), snappy (quick,
             overshoots and settles), smooth (no overshoot), linear.
  spring     { "hz": 4, "damping": 0.7 } tunes the settle (damping 0.2 bouncy to 1 none).
  anticipate a small move the other way first, as a fraction of the move (0.05 to 0.15).
  cascade    seconds between fingers in the move (default up to 0.035): closing runs pinky to
             index and then the thumb, opening frees the thumb, then index to pinky.
  pass       true makes the key a waypoint: the hand flows through it (passing near its pose)
             instead of slowing to a stop there. Use it for paths: an arc, a wave, a swing.
Per hand: every field but t can also go inside a hand of the key, timing that hand its own way
({ "t": 2, "move": 0.4, "hands": { "right": {..}, "left": { "move": 0.15, "ease": "snappy", .. } } });
what a hand leaves out comes from the key. A hand a key leaves out is not held by it: it keeps
moving between its own keys, so one hand can make a long move while the other makes several.
Moves overlap freely: a key can start moving before the previous one has arrived. A key at a
negative t starts the clip mid-move (the hand is already moving at 0). An ease or spring
written on a key applies to the move into that key only; spring overrides the ease's own.
Clip-wide: "idle": { "amount": 1, "seed": 0 } is a small breathing movement so a held hand
never freezes (0 turns it off); "follow": 1 scales how much relaxed fingers trail fast hand
moves.

Goals in clips: a goal held by two neighbouring keys (the same goal written in both) holds
through the move between them, every frame, so a pinch stays closed while the hand travels.
A goal only in the later key closes as the hand arrives; one only in the earlier key lets go
as the move starts. A hand's fingers never pass through each other or its palm: where a move
would, they slide past. A goal with only "vary" (no touch or aim) does the same in a pose
file: it keeps the parts it varies from passing through anything. Mid-move the two hands are
not pushed apart (frame by frame, a finger passing through the other hand would flip from one
side of it to the other) unless a held goal joins them: route a crossing yourself (a lifted
waypoint, fingers opened first); `check` reports each overlap with the keys either side.

Commands: `handkit check clip.json` (held goals, penetrations, joint speeds, pops, fingertips
leaving the camera's view, elbows in view; fix everything it reports; penetrations and pops
name the keys either side; "at limit" lines, joints held within 1 deg of a limit for over
0.3 s, are information: the pose asks a joint for more than it has), `handkit sheet clip.json
-n 12 --mesh` (a contact sheet at the clip's aspect; --at 1.2,1.3,1.4 for chosen times,
--background "#2b2e33" to match the video's), `handkit sample clip.json 1.5` (the pose
file at a time, to query or render; --wrist writes its placement as wrist, palm_facing and
fingers_toward), `handkit preview clip.json` (a quick MP4). To match a reference video:
`python tools/motion/compare.py ref.mp4 out.mp4 cmp` writes cmp.sheet.png (both at the same
times, in rows) and cmp.side.mp4 (side by side).
```

A fist that opens, points, makes an OK sign and turns while holding the pinch (`clips/demo_basics.json`; the OK pinch is held through the last move because both keys take it from the `ok` gesture):

```json
{
  "version": 1,
  "fps": 30,
  "camera": { "position": [0.05, 0.15, 0.55], "target": [0.1, 0.06, 0.0], "fov": 35 },
  "keys": [
    { "t": 0, "hands": { "right": { "from": "fist", "place": { "wrist": [0.1, -0.02, 0], "palm_facing": [0, 0, 1], "fingers_toward": [0, 1, 0] } } } },
    { "t": 1.2, "move": 0.5, "hands": { "right": { "from": "spread_palm" } } },
    { "t": 2.2, "move": 0.35, "ease": "snappy", "hands": { "right": { "from": "point" } } },
    { "t": 3.2, "move": 0.5, "hands": { "right": { "from": "ok" } } },
    { "t": 4.6, "move": 1.0, "hands": { "right": { "from": "ok", "place": { "wrist": [0.12, 0.0, 0.02], "palm_facing": [0.7, 0, 0.7], "fingers_toward": [-0.2, 1, 0] } } } }
  ]
}
```

## Reference

The text below is the output of `handkit explain`.

```text
World: meters and degrees. Y up, X right, +Z toward the default front camera.
Rest pose (all values 0): forearm from the elbow straight up (+Y, 0.26 m) to the wrist; hand flat
and straight up, palm facing +Z, back of the hand (nails) facing -Z. Right hand: thumb on +X,
then index, middle, ring, pinky toward -X. Left hand: the exact mirror (thumb on -X).
Joint values mean the same for both hands (anatomical); the rig mirrors the left hand.

Placement, per hand, one of:
  place: { wrist, palm_facing, fingers_toward }   the hand frame directly: wrist position, the
      direction the palm faces and the direction the fingers point (at wrist_flex/dev 0 the
      straight fingers point exactly there). The forearm follows from twist and the wrist angles.
      The two directions need not be at right angles or unit length: fingers_toward is kept
      and palm_facing is made perpendicular to it.
      The right thumb is on the side of fingers_toward x palm_facing (cross product); the left
      thumb on the opposite side. Example: palm_facing [0,-1,0], fingers_toward [1,0,0] puts the
      right thumb toward -Z and the left thumb toward +Z.
      Add forearm_toward: [x, y, z] (the direction from the wrist toward the elbow) to aim the
      forearm: it sets wrist_flex and wrist_dev (any written are replaced), so the hand stays
      exactly where placed and the arm comes in from that side. `query` reports the wrist
      angles as limit violations if the bend is more than a wrist can do.
      To place from the picture, with the pose's camera: screen: [x, y] (fractions of the
      frame: x 0 left edge to 1 right, y 0 top to 1 bottom) and depth (meters from the camera
      along its view direction) put the wrist there, instead of wrist; palm_facing,
      fingers_toward and forearm_toward work as with wrist. elbow_screen: [x, y] instead of
      forearm_toward aims the forearm so the elbow lands on that point of the frame (the
      farther of the two points a forearm's length from the wrist; out of reach, the forearm
      points toward it), e.g. [1.05, 0.9] brings the arm in from past the right edge. The
      frame's shape is the pose's top-level "size": [width, height] (default 16:9).
  place: { elbow, forearm_rotation_deg: [rx, ry, rz] }   rotates the rest pose about the elbow,
      matrix Rx*Ry*Rz (rz applied first, about world axes). For a left hand it rotates the
      mirrored rest pose.
  A hand written as { "mirror": "right", ... } copies the right hand's joint values and takes the
  mirror image of its placement across the plane x = 0; any keys given override the copy.

Chains: wrist = twist (about the forearm) -> wrist_dev -> wrist_flex. Fingers: knuckle spread
(rest fan + mcp_abd) -> mcp_roll -> mcp_flex -> pip_flex -> dip_flex. Thumb: base at (0.020, 0.018, 0.008)
in the hand frame (x toward the thumb, y toward the fingers, z out of the palm); rest
orientation = lean 35 deg toward the thumb side, lift 10 deg out of the palm, turn the pad 55
deg toward the index. Exactly, with rotations about the hand frame's axes applied left to right
(each about the already-rotated axes): Rz(-35 + cmc_flex) * Rx(10 + cmc_abd) *
Ry(-55 - cmc_rot). So positive cmc_flex undoes the lean, swinging the thumb toward the pinky
(at cmc_flex 35 it points along the fingers, before the lift); positive cmc_abd lifts it toward
+Z; positive cmc_rot turns the pad further toward the fingers. mcp_flex and ip_flex then rotate
about the thumb's own X axis, curling the tip toward the pad. At rest the right thumb points
along (0.57, 0.81, 0.17) and its pad faces (-0.73, 0.39, 0.57), toward the index and the viewer.
Run `handkit fk` on a pose to see every joint's position and axes.

Sizes: fingers (knuckle x, y; proximal, middle, distal length; radius): index (0.030, 0.093;
0.040, 0.025, 0.020; 9.5 mm), middle (0.010, 0.096; 0.045, 0.028, 0.021; 9.8 mm), ring
(-0.010, 0.091; 0.042, 0.027, 0.021; 9.2 mm), pinky (-0.029, 0.083; 0.033, 0.019, 0.018;
8.2 mm). Knuckles lie in the palm plane (z = 0). Each later segment is 10% thinner. Thumb
segments 0.046, 0.034, 0.027 m, radius 13.5, 10.5, 9.5 mm. Whole hand about 0.19 m.

Sites (surface points, named <hand>.<name>): for every digit tip (end of the fingertip), pad
(fingerprint), nail and knuckle (back of the MCP joint); for index to pinky side_radial and
side_ulnar (thumb side and pinky side of the fingertip) and dip_radial, dip_ulnar (sides of the
last joint); for the thumb side_palmar and side_dorsal (its two edges: the pad faces the index,
so one edge faces the palm side and one the back of the hand); for the hand palm_center,
palm_heel, wrist_back. Example: right.thumb.pad.
Where they sit, in each fingertip segment's frame (y along the segment from its base joint, z
toward the palm side, r its radius, l its length): tip (0, l + r, 0), pad (0, 0.55 l, r), nail
(0, 0.6 l, -r), side_radial (r, 0.5 l, 0), side_ulnar (-r, 0.5 l, 0); the thumb's side_palmar
and side_dorsal are the same two points as radial and ulnar. The palm surface is 12.5 mm in
front of the palm plane (palm_center (0, 0.055, 0.013) in the hand frame) and the back of the
hand 12.5 to 16 mm behind it.

Touching: a gap between surfaces of 0 to 3 mm. `query` lists contacts (across hands, anything
with a thumb, fingertips on the palm), penetrations (overlap over 2 mm), the nearest part to
each thumb tip, the surface gap for each pair in `checks` (a pair names sites or segments such
as right.index.middle; segments are metacarpal, proximal, middle, distal, and forearm), and
how well fingers aim at `probes`; `query --json` gives the same (with goals and fingertip
positions) as JSON.

Couplings (version 1 files): a dip_flex left out follows 2/3 of pip_flex.

Goals: "goals": [...] says what must be true; `handkit solve` changes values until it is and
writes them back, keeping changes small, joints inside their limits and fingers out of each
other (it may leave a penetration it cannot fix; it says so). If every goal already holds it
changes nothing; when a goal is still missed it writes the pose as far as it got and exits
with code 2. Note: `place` fixes the hand itself; wrist angles then swing the forearm, not
the hand.
  { "touch": [a, b] }   a and b are sites or segments. The surfaces meet (gap within 1 mm) and,
      for a site, the contact lands within 6 mm of it. Fingers do not roll about their own
      axis, so two pads seldom meet dead center: a contact a few mm toward the side of a pad
      is expected. Add "gap_mm": 15 to hold them that far apart instead (sites are then
      measured point to point, segments surface to surface).
  { "aim": "right.index", "at": [x, y, z] }   the finger's line (knuckle through tip) passes
      through the point, within 1 deg. The point is in world space, so an aim belongs to its
      placement: a hand-level "from" leaves a gesture's aims behind.
  "vary": [...] on a goal chooses what may change (default: every joint of each digit a touch
      names, plus palm_cup for ring and pinky and the wrist for palm sites; for aim, the wrist
      and that finger's knuckle). Patterns: right.thumb.* (one digit), right.index.pip_flex
      (one value), right.*.mcp_abd (that joint on every digit), right.wrist (twist, wrist_flex,
      wrist_dev), right.palm_cup, right.place (the hand's placement; right.place.position
      only moves it, right.place.rotation only turns it), right.* (everything on the hand).
      Placement is never varied unless named. A hand that mirrors the other varies the
      other's values, so the pair stays a mirror.
  Shapes that read right: OK circle = thumb.pad ~ index.tip; flat pinch = thumb.pad ~
  index.pad; finger heart = thumb.pad ~ index.dip_radial. Start from roughly the right shape:
  the solver finds the nearest solution, not the most natural one.

Gestures: `handkit gestures` lists a library of finished poses; `handkit gesture <name>` prints
one. "from": "<name>" at the top of a file starts from a whole gesture (keys you write win;
its goals are added to yours). On a hand, { "from": "ok" } takes that gesture's joint
values for this hand (not its placement), and its goals renamed to this hand.

Motion: `handkit explain clips` (keyed poses over time, checked and previewed as a whole).

Parameters (key, range in degrees, meaning):
  twist [-100, 100]: Turns the hand about the forearm's long axis. +90 makes the palm face the side the thumb pointed to at rest (right hand: +X, the thumb then points to -Z).
  wrist_flex [-80, 85]: Bends the hand toward the palm side; negative bends it toward the back of the hand.
  wrist_dev [-35, 35]: Bends the hand sideways toward the thumb (radial); negative toward the pinky. Applied before wrist_flex.
  palm_cup [0, 25]: Hollows the palm: the pinky knuckle comes 0.03*sin(cup) m forward and turns `cup` degrees toward the thumb, the ring knuckle half that. Use in fists, grips and thumb-to-pinky contact.
  thumb.cmc_flex [-30, 50]: Swings the whole thumb in the palm plane toward the pinky side (across the palm). Negative swings it further out.
  thumb.cmc_abd [-10, 75]: Lifts the whole thumb out of the palm plane toward the palm side, like opening the hand around a can.
  thumb.cmc_rot [-15, 60]: Rotates the thumb about its own long axis so the pad turns further toward the fingers (opposition). Applied after cmc_flex and cmc_abd.
  thumb.mcp_flex [-15, 65]: Bends the thumb's middle joint, curling the thumb toward its pad.
  thumb.ip_flex [-20, 85]: Bends the thumb's tip joint, curling toward its pad.
  index.mcp_abd [-15, 25]: Spreads the index away from the middle finger (toward the thumb); adds to the 4 deg rest fan.
  middle.mcp_abd [-15, 15]: Tilts the middle finger toward the index side; negative toward the ring.
  ring.mcp_abd [-15, 20]: Spreads the ring finger away from the middle finger (toward the pinky); adds to its -4 deg rest fan.
  pinky.mcp_abd [-15, 35]: Spreads the pinky away from the ring finger; adds to its -9 deg rest fan.
  index.mcp_roll [-10, 20]: Rolls the finger about its own length at the knuckle, turning the pad (and the direction the finger curls) toward the thumb; negative away. Real fingers roll a little as they curl toward the thumb, which is how a fingertip meets the thumb pad face to face.
  middle.mcp_roll [-10, 20]: Rolls the finger about its own length at the knuckle, turning the pad (and the direction the finger curls) toward the thumb; negative away. Real fingers roll a little as they curl toward the thumb, which is how a fingertip meets the thumb pad face to face.
  ring.mcp_roll [-10, 25]: Rolls the finger about its own length at the knuckle, turning the pad (and the direction the finger curls) toward the thumb; negative away. Real fingers roll a little as they curl toward the thumb, which is how a fingertip meets the thumb pad face to face.
  pinky.mcp_roll [-10, 30]: Rolls the finger about its own length at the knuckle, turning the pad (and the direction the finger curls) toward the thumb; negative away. Real fingers roll a little as they curl toward the thumb, which is how a fingertip meets the thumb pad face to face.
  index.mcp_flex [-30, 95]: Bends the finger at the knuckle toward the palm; negative hyperextends.
  index.pip_flex [-5, 110]: Bends the middle joint toward the palm.
  index.dip_flex [-10, 85]: Bends the fingertip joint toward the palm. In a version 1 file, left out it follows 2/3 of pip_flex (the tendon coupling of real fingers).
  middle.mcp_flex [-30, 95]: Bends the finger at the knuckle toward the palm; negative hyperextends.
  middle.pip_flex [-5, 110]: Bends the middle joint toward the palm.
  middle.dip_flex [-10, 85]: Bends the fingertip joint toward the palm. In a version 1 file, left out it follows 2/3 of pip_flex (the tendon coupling of real fingers).
  ring.mcp_flex [-30, 95]: Bends the finger at the knuckle toward the palm; negative hyperextends.
  ring.pip_flex [-5, 110]: Bends the middle joint toward the palm.
  ring.dip_flex [-10, 85]: Bends the fingertip joint toward the palm. In a version 1 file, left out it follows 2/3 of pip_flex (the tendon coupling of real fingers).
  pinky.mcp_flex [-30, 95]: Bends the finger at the knuckle toward the palm; negative hyperextends.
  pinky.pip_flex [-5, 110]: Bends the middle joint toward the palm.
  pinky.dip_flex [-10, 85]: Bends the fingertip joint toward the palm. In a version 1 file, left out it follows 2/3 of pip_flex (the tendon coupling of real fingers).
```
