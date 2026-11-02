# Hand pose spec (experiment rig)

You pose one or two hands (each with a forearm) by writing a JSON file of joint angles.
All angles are in **degrees**, all positions in **meters**. Missing values are 0.

## World

- Y is up, X is to the right, +Z points toward the default front camera (the viewer).
- The renderer shows four views: FRONT (camera on +Z looking toward -Z), RIGHT SIDE (camera on
  +X), TOP (camera on +Y, with -Z at the top of the image), and either a three-quarter view or
  your own `camera`.

## Rest pose (every value 0)

The forearm runs from the elbow straight up (+Y), 0.26 m long, to the wrist. The hand is flat
and continues straight up: fingers extended toward +Y, **palm facing +Z** (toward the viewer),
back of the hand (nails) facing -Z.

- **Right hand:** thumb on the +X side, lying roughly in the palm plane, leaning 35° away from
  the index finger. Index finger nearest the thumb, then middle, ring, pinky toward -X.
- **Left hand:** exact mirror image: thumb on the -X side, pinky toward +X.

So at rest, a viewer at +Z sees both palms, like someone holding both hands up with palms out.

## Hand dimensions (right hand, hand frame)

Hand frame: origin at the wrist joint, +Y toward the fingers, +Z out of the palm, +X toward
the thumb (left hand: -X toward the thumb).

| finger | knuckle (MCP) at (x, y) | proximal, middle, distal length | radius |
| --- | --- | --- | --- |
| index | (0.030, 0.093) | 0.040, 0.025, 0.020 | 9.5 mm |
| middle | (0.010, 0.096) | 0.045, 0.028, 0.021 | 9.8 mm |
| ring | (-0.010, 0.091) | 0.042, 0.027, 0.021 | 9.2 mm |
| pinky | (-0.029, 0.083) | 0.033, 0.019, 0.018 | 8.2 mm |

Thumb: its base (CMC) joint is at (0.020, 0.018, 0.008), segments 0.046 (metacarpal, inside
the fleshy base of the thumb), 0.034 (proximal), 0.027 (distal). At rest the fingers fan
slightly (index 4° toward the thumb, ring 4° and pinky 9° away). A whole hand is ~0.19 m long.

## JSON

```json
{
  "hands": {
    "right": {
      "elbow": [0.1, -0.3, 0],
      "forearm_rotation_deg": [0, 0, 0],
      "twist": 0, "wrist_flex": 0, "wrist_dev": 0, "palm_cup": 0,
      "thumb":  { "cmc_flex": 0, "cmc_abd": 0, "cmc_rot": 0, "mcp_flex": 0, "ip_flex": 0 },
      "index":  { "mcp_flex": 0, "mcp_abd": 0, "pip_flex": 0, "dip_flex": 0 },
      "middle": { "mcp_flex": 0, "mcp_abd": 0, "pip_flex": 0, "dip_flex": 0 },
      "ring":   { "mcp_flex": 0, "mcp_abd": 0, "pip_flex": 0, "dip_flex": 0 },
      "pinky":  { "mcp_flex": 0, "mcp_abd": 0, "pip_flex": 0, "dip_flex": 0 }
    },
    "left": { "...": "same keys" }
  },
  "camera": { "position": [0, 0, 0.6], "target": [0, 0, 0], "fov": 35 }
}
```

Leave out a hand to show only one. `camera` is optional (a perspective camera for the 4th view).

## Placing the forearm

- `elbow`: world position of the elbow.
- `forearm_rotation_deg: [rx, ry, rz]`: rotates the whole rest pose (forearm + hand) about the
  elbow. Three.js Euler order `'XYZ'`: the matrix is Rx·Ry·Rz, i.e. it first rotates by `rz`
  about world Z, then by `ry` about world Y, then by `rx` about world X. Right-handed: positive
  angles turn counter-clockwise when looking from the + end of the axis toward the origin.
  - `[90, 0, 0]`: forearm points toward +Z (at the viewer), palm faces -Y (down).
  - `[-90, 0, 0]`: forearm points toward -Z (away from the viewer), palm faces +Y (up).
  - `[0, 0, -90]`: forearm points toward +X, palm still faces +Z.
  - `[0, 0, 90]`: forearm points toward -X.
  - `[0, 180, 0]`: forearm still up, palm faces -Z (the viewer sees the back of the hand).
- For a left hand the rotation is applied to its (mirrored) rest pose the same way.

## Joints

Ranges are the rig's joint limits; values outside them are reported as violations.

| key | range | meaning (positive direction) |
| --- | --- | --- |
| `twist` | -100..100 | turns the hand about the forearm's long axis. +90 makes the palm face the side the thumb pointed to at rest (right hand: +X, then the thumb points toward -Z; left hand: -X) |
| `wrist_flex` | -70..80 | bends the hand toward the palm side (negative: toward the back of the hand) |
| `wrist_dev` | -25..35 | bends the hand sideways toward the thumb (radial); negative toward the pinky |
| `palm_cup` | 0..25 | hollows the palm: the ring and pinky knuckles come forward and the pinky turns toward the thumb (used in fists, grips and when the pinky meets the thumb) |
| finger `mcp_flex` | -30..95 | bends the finger at the knuckle toward the palm |
| finger `mcp_abd` | index -15..25, middle -15..15, ring -15..20, pinky -15..35 | spreads the finger away from the middle finger (middle: toward the index). Negative squeezes it toward the middle finger |
| finger `pip_flex` | -5..110 | bends the middle joint toward the palm |
| finger `dip_flex` | -10..85 | bends the fingertip joint toward the palm. In real hands it usually is about 2/3 of `pip_flex` |
| thumb `cmc_flex` | -20..50 | swings the whole thumb in the palm plane toward the pinky side (across the palm). Negative swings it further out to the side |
| thumb `cmc_abd` | -10..75 | lifts the whole thumb out of the palm plane toward the palm side (+Z at rest), like opening the hand to hold a can |
| thumb `cmc_rot` | -15..60 | rotates the thumb about its own long axis so its pad turns to face the fingertips (opposition). Use it with `cmc_abd` and `cmc_flex` when the thumb touches a fingertip |
| thumb `mcp_flex` | -15..65 | bends the thumb's middle joint, curling toward its pad |
| thumb `ip_flex` | -20..85 | bends the thumb's tip joint, curling toward its pad |

The finger joints chain in this order: knuckle spread (abd) → knuckle bend → middle joint →
tip joint. The thumb chain is cmc_flex (in palm plane) → cmc_abd (out of palm) → cmc_rot (own
axis) → mcp → ip. Each child moves with its parent.

## Report

`node render.mjs pose.json [out.png]` writes the image and prints a JSON report:

- `joint_limit_violations`: values outside the ranges above.
- `penetrations`: pairs of segments from different fingers or hands that overlap by more than
  4 mm (fingers passing through each other).
- `fingertip_world_m`: world position of the tip of each finger.
- `fingertip_surface_gaps_mm`: distance between fingertip surfaces for the thumb against each
  finger of the same hand, and every fingertip pair across two hands. 0 to 3 mm means touching;
  negative means overlapping.
