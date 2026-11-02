# m1-cli-opus run notes

Call counts are handkit invocations. Each count includes one final `query` pass over all 7 files. `fk` calls were piped through grep to read a few sites.

| Task | query | render | fk | confidence | hardest part |
|---|---|---|---|---|---|
| T1 finger heart | 9 | 5 | 6 | 70 | Getting the thumb pad onto the radial side of the index DIP while the thumb and index still stick up past the curled fingers. Seen straight from behind, the crossing is hidden, so the hero camera is set to the thumb side. |
| T2 camera frame | 4 | 1 | 2 | 85 | The thumb lean tops out at 55 deg, so the L needs index `mcp_abd -15` plus thumb hyperextension. Placing the second hand was easy: the left hand is the right hand inverted through the frame centre, so one contact solves both. |
| T3 heart hands | 22 | 5 | 7 | 75 | Finding a placement where the fingers arch over the top and the thumbs point down, which is a rotated "C grip". Changes are strongly coupled: rotating the hand to fix the thumbs moves the fingertips. I used `mirror` so only the right hand had to reach x=0. The bottom V is still shallower than in the photo. |
| T4 offering palm | 2 | 2 | 0 | 70 | Nothing hard to pose. I'm unsure how well it matches the painting's foreshortening, and from above the palm reads thin in the render. |
| T5 pointing | 12 | 2 | 6 | 85 | Aim was solved analytically from fk (knuckle to target), finishing at 0.7 mm. Laying the thumb over the curled middle phalanx took several abd/flex tries. |
| T6 OK sign | 17 | 3 | 16 | 65 | Getting thumb tip to index tip. The thumb's reach sphere and its curl direction (toward -X and -Z) kept making it pass through the index. In the end I moved the index tip to the thumb instead. The circle is visible from the side or three-quarter view, but edge-on from the front. |
| T7 bunny ears | 5 | 1 | 1 | 85 | Reused what worked for the T5 thumb. The site check reads 15 mm even though the capsules touch (1.5 to 2.9 mm). |

Totals: 71 query, 19 render, 38 fk.

## Confusing or missing

- **Site checks vs capsule contacts.** `checks` only take sites, and there is no site on the back of a proximal or middle phalanx. So "thumb resting over the curled middle/ring finger" (T5, T7) can't be written as a check that reads near 0, even when `contacts` shows the capsules touching. T2 has the same problem: the side_radial site sits behind the tip, so the check shows 7.4 mm while the capsules are at 0.2 mm. A part-to-part check such as `["right.thumb.distal", "right.middle.middle"]` would fix it.
- **Which way the thumb moves.** cmc_flex, cmc_abd and cmc_rot are described in the hand frame, but once the hand is placed at an angle I couldn't predict where a change would move the thumb tip in world space. In T3, cmc_flex barely changed the thumb's angle in the image. Thumb mcp/ip flex curls toward the pad, which (after cmc_rot) mostly points across and out of the palm, and I misjudged it more than once.
- **Every iteration needed `fk | grep`.** `query` prints fingertip positions only. I nearly always also needed pad, nail and tip positions with their facing directions, and the thumb joint directions.
- **Mirrored hands double the output.** Every penetration and contact with a mirrored left hand is listed twice (A~B and B~A).
- **Orthogonal `place` vectors.** The spec doesn't say whether `palm_facing` and `fingers_toward` must be orthogonal or are made orthogonal for you. I always made them orthogonal by hand.
- **Probes don't name a finger.** A probe doesn't say which finger it is for. The output picked the index, which happened to be right.
- **No "seen from" helper.** Writing a camera for "photographer's view" or "seen from behind" took guessing.

## Helper that would have saved the most work

A small solver, e.g. `handkit reach pose.json --touch right.thumb.tip right.index.tip --vary thumb.*`, would have saved the most work. It would change the named parameters to close a contact gap without penetration and print the new values. Most of my query calls (T3, T6, T1) were hand-run 1D line searches on one gap. A cheaper version would also help: `handkit nudge pose.json right.thumb.tip`, printing how far one site moves in world space for +1 deg of each parameter.
