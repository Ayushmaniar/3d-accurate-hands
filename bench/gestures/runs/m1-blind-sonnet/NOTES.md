# m1-blind-sonnet notes

## Confidence and hardest part
- T1 finger heart: 30. Getting the thumb pad onto the index side near the PIP/DIP, with the thumb crossing the palm, from angles alone.
- T2 camera frame: 40. The thumb cannot reach a true 90 deg from the index in the rig (max about 75 deg), so I turned the palm to face down/up and used cmc_abd 60 so the thumb points along the palm normal. Tip contacts were solved by hand arithmetic (they came out consistent by symmetry), but thumb tip position is approximate.
- T3 heart hands: 15. Getting both fingertips to meet at top centre AND both thumb tips to meet at the bottom; my estimate has thumb tips about 6 cm apart.
- T4 offering palm: 70. Mostly orientation; forgiving.
- T5 pointing: 65. Index line passes through the target by construction (accounted for the 0.03 m knuckle offset, cancelled the 4 deg fan with mcp_abd -4); risk is sign/convention errors and the thumb-over-middle placement.
- T6 OK sign: 30. Circle closure: index tip and thumb tip must meet; thumb reach was estimated at about 0.087 m of 0.107 m.
- T7 bunny ears: 60. Thumb reaching over the curled ring and pinky, and sign of the V spread.

## Ambiguities and guesses
- Whether `place` requires palm_facing and fingers_toward to be exactly perpendicular, or orthogonalises them. I made them perpendicular.
- Whether the 4 deg index rest fan is in the hand frame and tilts toward +x (thumb side); I assumed so and cancelled it with -4 in T5.
- The exact thumb direction formula with cmc_flex/abd/rot: I assumed direction ~ (sin L cos T, cos L cos T, sin T) in the hand frame, L = 35 - cmc_flex, T = 10 + cmc_abd. cmc_rot effect on tip position is unknown (it spins about the long axis).
- Direction of mcp_flex/ip_flex curl of the thumb ("toward the pad") in world terms is unclear.
- Whether left-hand `place` vectors are written in world space unmirrored (I assumed yes for explicit left hands, and mirrored x for `mirror`).
- The "back of hand toward viewer" views assume the viewer is at +Z (default camera).
- Site names I guessed: right.index.dip_radial, right.middle.dip_radial, right.ring.nail, tip, pad.
- Which wrist/forearm angles come out natural when the placement fixes the hand frame; I left wrist angles at 0 except T4.
- T3: photographer's view treated as camera at +Z; hands' palm normals chosen so the thumb (nearly along the palm normal when heavily abducted) points down.
