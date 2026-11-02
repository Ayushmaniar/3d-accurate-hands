# loop-opus notes

Method: I wrote my own forward-kinematics model of the rig from SPEC.md (`fk.mjs`). I checked it
against the `fingertip_world_m` that every render reports, and solved for joint angles with a small
coordinate-descent IK script (`solve.mjs`: point, axis, relative-offset, distance and
"line passes through point" goals). The finger chain in my model matched the renderer to the
millimetre from the first render. The thumb was 10–15 mm off until I fitted its hidden rest
orientation from four T1 renders (`fit3.mjs`, 0.7 mm RMS afterwards). From then on, most renders
were a visual check of a solved pose, not trial and error.

## Per task

| task | renders | confidence | hardest part |
| --- | --- | --- | --- |
| T1 finger heart | 5 | 65 | Thumb placement. My thumb model was wrong at first: two renders put the thumb through the index or next to it. After fitting, the solved pose was right, but "pad on the side of the index, tip just past it" is hard to pin down as numbers. The framing is a little looser than the photo. |
| T2 camera frame | 3 | 75 | Getting a near-90° L. The rig's thumb sits only about 35–45° from the index, so I needed cmc_flex −20, mcp/ip extension, index abd −15 and twist to bring the thumb back into the frame plane. I also had to place two hands so that both corners meet without one tip going through the other (render 2 had a 9.5 mm thumb-index penetration). |
| T3 heart hands | 5 distinct poses (7 render calls: I ran 2 more on unchanged poses only to read the full report after my grep dropped it; one of the 5 was wasted because my solver left the pinky pip at its old value) | 60 | Finding a hand orientation where the fingers curl inside the picture plane. The palms have to face inward and away (forearm ry ≈ ±125) so the curl reads as a lobe. The first try gave a flat bottom and finger "claws". The lobes are still somewhat angular, and the backs of the hands show less than in the photo. |
| T4 offering palm | 2 | 75 | Composing the forearm Euler angles for "toward the viewer, palm up" ([102,180,0]). After that it was taste: how much curl, and the thumb rising a little above the palm. |
| T5 pointing | 2 | 90 | Nothing hard once IK existed. The index line passes 0.3 mm from the target (MCP→tip line from my FK, which matches the reported tip exactly). Render 2 only removed a 4.9 mm pinky-into-palm penetration. |
| T6 OK sign | 2 | 70 | The ring was easy to close (gap 0.5 mm). But with the palm squarely facing the viewer, the index's curl plane, and so the O, is seen nearly edge-on. I added twist −25 so the O shows from the front and added a 3/4 camera. |
| T7 bunny ears | 1 | 85 | Placing the thumb pad over the curled ring and pinky. Solved directly with the fitted thumb model. |

## What in the spec was ambiguous

- **Thumb rest frame.** The thumb's rest orientation is underspecified. "Lying roughly in the palm plane, leaning 35°" does not say where the pad faces, so it does not say which way mcp/ip curl. It also does not say there is a built-in out-of-plane tilt. The fit says: base = Rz(−35)·Rx(+10) (about 10° of built-in abduction), with the pad turned about 55° from the palm normal toward the index. cmc_rot rotates the pad further the same way (Ry(−55−rot) in my frame). This cost the most renders.
- **cmc_rot sign.** "Pad turns to face the fingertips" depends on where the pad faces at 0.
- **Wrist order.** The order of twist / wrist_flex / wrist_dev is not stated. Twist→flex→dev happened to match.
- **Fingertip surface gaps.** How the gaps are computed is not given. Empirically, touching means tip points about 17 mm apart (distal radius about 0.85× the listed radius).
- **palm_cup.** How much palm_cup moves the ring and pinky knuckles is not given. My FK ignored it, which caused about 3 mm of error.
- **"Photographer's view".** For T3 it is not obvious whether the backs or the palms of the hands face the camera. The photo implies the backs.
- **"Thumb tip just past it"** (T1). It is unclear which direction "past" means.
- **Left-hand Euler.** The spec says the left hand uses "the same rotation applied to the mirrored rest pose". I needed [rx, −ry, −rz] for a mirror-symmetric pose; it worked, but I had to derive it.

## Learned only from renders

- Finger FK is exactly as I modelled it. Abd comes before flex, about the hand Z axis. The rest fan is +4 / 0 / −4 / −9 toward the thumb. Flex is about the local X axis, toward the palm.
- The thumb rest frame above (built-in 10° abduction, pad pre-rotated 55° toward the index).
- The nail discs show at the very ends of the fingertips even from the palm side. That made "which way is it facing" reads unreliable on small views.
- The renderer auto-frames each fixed view, so absolute scale differs between views.

## Helpers that would have saved the most iterations

1. **Exposed FK.** A `--joints` flag (or JSON field) that dumps world positions of every joint and the local frames, including the thumb, without rendering. That alone would have removed the 3–4 thumb-calibration renders.
2. **Contact / IK goals.** For example `thumb.touch: {finger: "index", where: "tip" | "dip-radial-side", hand: "left"}`, or `point_at: [x,y,z]` for a finger, solved by the rig. T1, T2, T3, T5 and T6 are all contact or aiming constraints.
3. **Orientation by intent.** Something like `palm_normal: [x,y,z], fingers_dir: [x,y,z]` instead of Euler XYZ plus twist. Building "forearm toward the viewer, palm up" or "palm facing inward-away" from Euler angles is error-prone.
4. **`mirror: "right"` for the left hand**, for symmetric two-hand gestures (T3).
5. **Presets** (fist, L, relaxed curl) that take a scalar amount, plus a "curl" helper that sets dip ≈ 2/3 pip.
6. **The full report on stdout with all tips and joint positions**, and an option to render the report only, so checking a number does not cost a render.
