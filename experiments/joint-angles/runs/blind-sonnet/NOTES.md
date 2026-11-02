# Blind joint-angle run (sonnet)

## Confidence and hardest part

- T1 finger heart: 25. Thumb reach: the thumb (0.107 m) barely reaches the index mid-phalanx, so I curled the index hard (60/60/30) and guessed thumb bend directions; crossing geometry is unverified.
- T2 camera frame: 35. I tilted both hands about the index axis (rx -70) so the abducted thumb lies in the frame plane, and solved the wrist offset so both tip pairs coincide (~4 cm depth mismatch left over). The L is only about 70 deg, not 90.
- T3 heart hands: 30. Tip x-positions were balanced by hand (wrist_flex -30, palm facing down), but heart is only ~6 cm tall and finger-to-finger tip overlap at the center is likely (penetrations).
- T4 offering palm: 65. Orientation [105,180,0] was easy to derive (toward viewer, 15 deg down, palm up, thumb on viewer's left); finger curls are just plausible values.
- T5 pointing: 75. Solved the rotation analytically so the index line (offset 0.03 m from the forearm axis) passes through the target, assuming mcp_abd -4 cancels the 4 deg rest fan; thumb over middle finger is a guess.
- T6 OK sign: 40. Index tip and thumb tip placed ~19 mm apart by hand FK; thumb bend direction conventions uncertain; hand yawed -35 deg to show the circle.
- T7 bunny ears: 55. V spread easy; thumb pinning ring/pinky depends on guessing where the thumb pad lands (computed tip near the ring finger back, maybe 2 cm too high in z).

## Ambiguities / guesses

- Direction of mcp_flex/ip_flex for the thumb: assumed they curl toward the pad, with the pad normal starting at +Z (palm side) and tilting with cmc_abd.
- Sign and meaning of cmc_rot (assumed positive turns the pad toward the fingertips/ulnar side); I mostly kept it small.
- Whether mcp_abd adds to the 4/4/9 deg rest fan (assumed additive, so index -4 is exactly straight).
- Whether cmc_abd lifts about an axis perpendicular to the current in-plane thumb direction (assumed so), and what happens to the thumb when cmc_flex is negative with large abd.
- Left hand local frame: assumed the same rotation matrix applies and thumb axis is mirrored (local -X); the 'palm_cup' direction for the left hand is assumed mirrored.
- Where fingertip positions are measured (end of last segment vs pad surface); I assumed centers must be roughly 2 radii apart for contact.
- Wrist_flex axis for rotated hands (assumed hand-local X, so in T3 negative flex tilts fingers upward).
- Finger flex angles are assumed cumulative along the chain (mcp+pip+dip) in one plane, with positive toward +Z palm side.
- Camera 'fov' assumed vertical degrees; 'target' semantic assumed look-at.
