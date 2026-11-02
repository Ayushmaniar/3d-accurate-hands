# blind-opus notes

No code was run. I worked out forward kinematics by hand on paper (2-3 significant figures). I set up the
symmetric two-hand tasks so that the contact conditions reduce to one or two scalar equations.

## Per task

| task | confidence | hardest part |
| --- | --- | --- |
| T1 finger heart | 40 | Getting the thumb pad onto the radial side of the index middle phalanx. The thumb (0.107 m) only just reaches, so it has to be nearly straight, and whether it touches or penetrates depends on the thumb radius, which the spec doesn't give. |
| T2 camera frame | 50 | Thumb in-plane range stops at 55° from the hand axis (cmc_flex -20), so the "L" is only ~66° even with index abd -15. I tilted each hand 23° to split the error. I placed the hands by point-inversion symmetry (right = -left), which makes both contacts one equation. |
| T3 heart hands | 40 | Interpreting the photo (I read it as palms facing each other and away from the camera, radial edges toward the camera). The thumbs must point roughly 90°+ from the hand axis, which needs cmc_abd 60. Placement uses mirror symmetry so that only the tip x-coordinates matter. |
| T4 offering palm | 65 | Building the forearm Euler angles so that palm up, forearm toward the viewer and slightly down are all satisfied. Twist can't do the 180° roll, so it all goes into forearm_rotation. |
| T5 pointing | 70 | The 0.030 m lateral offset of the index line from the forearm axis (I corrected for it, which is about 1.6°). The result depends entirely on mcp_abd -4 exactly cancelling the 4° rest fan and on the index knuckle sitting at z=0. Palm down needed Euler [110.97, -31.70, 126.12]. |
| T6 OK sign | 45 | Thumb-index tip contact. The thumb's flex direction depends on the unknown pad orientation and cmc_rot convention. I estimate a gap of about 0-5 mm if my thumb model is right. |
| T7 bunny ears | 50 | Thumb lying over the curled ring middle phalanx without penetrating it. The predicted overlap of about 2 mm depends on palm_cup's unspecified knuckle shift and on the thumb radius. |

## Spec ambiguities / guesses

1. **Thumb pad orientation at rest and flex axis.** The spec doesn't say whether mcp_flex/ip_flex curl toward the palm normal (+Z, like the fingers) or across the palm. I assumed the pad faces +Z at rest (same frame as the fingers) and that positive cmc_rot turns the pad toward the index/radial-to-ulnar side. This choice moves thumb tips by more than 15 mm.
2. **Thumb CMC axes.** I assumed cmc_flex rotates about the palm normal and cmc_abd rotates about the in-palm axis perpendicular to the thumb (so the thumb rises toward +Z, rotated by its current cmc_flex). The sign convention of cmc_rot for the left hand is assumed mirrored.
3. **Thumb radius is not given.** Finger radii are listed but the thumb's isn't. I assumed about 10 mm.
4. **Does mcp_abd add linearly to the rest fan?** I assumed index abd -4 makes the index exactly parallel to the hand +Y axis. T5 depends on this.
5. **Knuckle z coordinate.** The MCP table gives (x, y) only. I assumed z=0 (the thumb CMC has z=0.008, so the knuckles might not be at 0).
6. **Order of wrist joints** (twist vs wrist_flex vs wrist_dev) and the pivot of wrist_dev are unspecified. I avoided combining them (all poses use at most wrist_flex alone).
7. **palm_cup is qualitative only.** How far the ring and pinky knuckles move and how much the pinky turns isn't given, so it affects contacts in T5, T7 and the fists in ways I can't predict.
8. **Where "fingertip" is measured** (end of the distal bone vs the end of the capsule) and how the surface gap is computed are unclear. I assumed bone end and gap = centre distance minus the two radii.
9. **"Photographer's view" for T2 and T3.** I assumed the owner's eyes are at +Z looking toward -Z, with the left hand appearing on the left. For T3 the photo is a backlit silhouette, so which side of the hand faces the camera was a judgement call.
10. **Camera fov.** I assumed it's vertical, in degrees.
