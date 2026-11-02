# Handkit pose spec (version 1)

You pose one or two hands, each with a forearm, by writing a JSON pose file of joint angles.
Angles are degrees, positions meters; anything left out is 0. Unknown keys are errors.

## Workflow

1. Write `pose.json` (format below).
2. `handkit query pose.json`: limit violations, fingers passing through each other, contacts,
   the nearest part to each thumb tip, the gaps for your `checks`, aim at your `probes`, and
   fingertip positions. Text only, no image. Run it after every edit.
3. `handkit render pose.json --views front,hero --size 384`: a small multi-view PNG to judge
   the look. Views: front, back, side, top, three_quarter, hero (your `camera`).
4. `handkit fk pose.json`: every joint's position and axes and every site, when you need exact
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
  "checks": [["right.thumb.pad", "right.index.pad"]],
  "probes": { "target": [0.0, 0.3, -0.5] },
  "camera": { "position": [0, 0.05, 0.6], "target": [0, 0.05, 0], "fov": 35 }
}
```

`checks`, `probes` and `camera` are optional. Leave out a hand to show only one.

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
      The right thumb is on the side of fingers_toward x palm_facing (cross product); the left
      thumb on the opposite side. Example: palm_facing [0,-1,0], fingers_toward [1,0,0] puts the
      right thumb toward -Z and the left thumb toward +Z.
  place: { elbow, forearm_rotation_deg: [rx, ry, rz] }   rotates the rest pose about the elbow,
      matrix Rx*Ry*Rz (rz applied first, about world axes). For a left hand it rotates the
      mirrored rest pose.
  A hand written as { "mirror": "right", ... } copies the right hand's joint values and takes the
  mirror image of its placement across the plane x = 0; any keys given override the copy.

Chains: wrist = twist (about the forearm) -> wrist_dev -> wrist_flex. Fingers: knuckle spread
(rest fan + mcp_abd) -> mcp_flex -> pip_flex -> dip_flex. Thumb: base at (0.020, 0.018, 0.008)
in the hand frame (x toward the thumb, y toward the fingers, z out of the palm); rest
orientation = lean 35 deg toward the thumb side, lift 10 deg out of the palm, turn the pad 55
deg toward the index; then cmc_flex adds to the lean (toward the pinky), cmc_abd to the lift,
cmc_rot to the pad turn; mcp_flex and ip_flex curl toward the pad. At rest the right thumb
points along (0.57, 0.81, 0.17) and its pad faces (-0.73, 0.39, 0.57), toward the index and the
viewer. Run `handkit fk` on a pose to see every joint's position and axes.

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

Touching: a gap between surfaces of 0 to 3 mm. `query` lists contacts (across hands, anything
with a thumb, fingertips on the palm), penetrations (overlap over 2 mm), the nearest part to
each thumb tip, site-to-site gaps for the pairs in `checks`, and how well fingers aim at
`probes`.

Couplings (version 1 files): a dip_flex left out follows 2/3 of pip_flex.

Parameters (key, range in degrees, meaning):
  twist [-100, 100]: Turns the hand about the forearm's long axis. +90 makes the palm face the side the thumb pointed to at rest (right hand: +X, the thumb then points to -Z).
  wrist_flex [-70, 80]: Bends the hand toward the palm side; negative bends it toward the back of the hand.
  wrist_dev [-25, 35]: Bends the hand sideways toward the thumb (radial); negative toward the pinky. Applied before wrist_flex.
  palm_cup [0, 25]: Hollows the palm: the pinky knuckle comes 0.03*sin(cup) m forward and turns `cup` degrees toward the thumb, the ring knuckle half that. Use in fists, grips and thumb-to-pinky contact.
  thumb.cmc_flex [-20, 50]: Swings the whole thumb in the palm plane toward the pinky side (across the palm). Negative swings it further out.
  thumb.cmc_abd [-10, 75]: Lifts the whole thumb out of the palm plane toward the palm side, like opening the hand around a can.
  thumb.cmc_rot [-15, 60]: Rotates the thumb about its own long axis so the pad turns further toward the fingers (opposition). Applied after cmc_flex and cmc_abd.
  thumb.mcp_flex [-15, 65]: Bends the thumb's middle joint, curling the thumb toward its pad.
  thumb.ip_flex [-20, 85]: Bends the thumb's tip joint, curling toward its pad.
  index.mcp_abd [-15, 25]: Spreads the index away from the middle finger (toward the thumb); adds to the 4 deg rest fan.
  middle.mcp_abd [-15, 15]: Tilts the middle finger toward the index side; negative toward the ring.
  ring.mcp_abd [-15, 20]: Spreads the ring finger away from the middle finger (toward the pinky); adds to its -4 deg rest fan.
  pinky.mcp_abd [-15, 35]: Spreads the pinky away from the ring finger; adds to its -9 deg rest fan.
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
