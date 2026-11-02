//! Text for `handkit explain`: the conventions and every parameter, from the same table the
//! limit checks use.

use crate::params::{self, PARAMS};

pub const CONVENTIONS: &str = "\
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
  place: { elbow, forearm_rotation_deg: [rx, ry, rz] }   rotates the rest pose about the elbow,
      matrix Rx*Ry*Rz (rz applied first, about world axes). For a left hand it rotates the
      mirrored rest pose.
  A hand written as { \"mirror\": \"right\", ... } copies the right hand's joint values and takes the
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
how well fingers aim at `probes`.

Couplings (version 1 files): a dip_flex left out follows 2/3 of pip_flex.

Goals: \"goals\": [...] says what must be true; `handkit solve` changes values until it is and
writes them back, keeping changes small, joints inside their limits and fingers out of each
other (it may leave a penetration it cannot fix; it says so). If every goal already holds it
changes nothing. Note: `place` fixes the hand itself; wrist angles then swing the forearm, not
the hand.
  { \"touch\": [a, b] }   a and b are sites or segments. The surfaces meet (gap within 1 mm) and,
      for a site, the contact lands within 6 mm of it. Fingers do not roll about their own
      axis, so two pads seldom meet dead center: a contact a few mm toward the side of a pad
      is expected. Add \"gap_mm\": 15 to hold them that far apart instead (sites are then
      measured point to point, segments surface to surface).
  { \"aim\": \"right.index\", \"at\": [x, y, z] }   the finger's line (knuckle through tip) passes
      through the point, within 1 deg. The point is in world space, so an aim belongs to its
      placement: a hand-level \"from\" leaves a gesture's aims behind.
  \"vary\": [...] on a goal chooses what may change (default: every joint of each digit a touch
      names, plus palm_cup for ring and pinky and the wrist for palm sites; for aim, the wrist
      and that finger's knuckle). Patterns: right.thumb.* (one digit), right.index.pip_flex
      (one value), right.*.mcp_abd (that joint on every digit), right.wrist (twist, wrist_flex,
      wrist_dev), right.palm_cup, right.place (the
      hand's placement), right.* (everything on the hand). A hand that mirrors the other varies
      the other's values, so the pair stays a mirror.
  Shapes that read right: OK circle = thumb.pad ~ index.tip; flat pinch = thumb.pad ~
  index.pad; finger heart = thumb.pad ~ index.dip_radial. Start from roughly the right shape:
  the solver finds the nearest solution, not the most natural one.

Gestures: `handkit gestures` lists a library of finished poses; `handkit gesture <name>` prints
one. \"from\": \"<name>\" at the top of a file starts from a whole gesture (keys you write win;
its goals are added to yours). On a hand, { \"from\": \"ok\" } takes that gesture's joint
values for this hand (not its placement), and its goals renamed to this hand.";

pub fn all() -> String {
    let mut s = String::from(CONVENTIONS);
    s.push_str("\n\nParameters (key, range in degrees, meaning):\n");
    for p in PARAMS {
        s.push_str(&format!("  {} [{}, {}]: {}\n", p.key, p.min, p.max, p.doc));
    }
    s
}

pub fn one(key: &str) -> Result<String, String> {
    let k = key.trim_start_matches("right.").trim_start_matches("left.");
    match params::find(k) {
        Some(p) => Ok(format!("{} [{}, {}]: {}", p.key, p.min, p.max, p.doc)),
        None => Err(format!(
            "unknown parameter \"{key}\". Known: {}",
            PARAMS.iter().map(|p| p.key).collect::<Vec<_>>().join(", ")
        )),
    }
}
