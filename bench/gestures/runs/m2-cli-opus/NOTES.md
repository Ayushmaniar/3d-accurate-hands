# m2-cli-opus notes

Every pose was built by hand from `docs/POSE_SPEC.md`. I did not use the gesture library or `"from"`, and I did not run `fk`. All 7 files end with every goal `ok`, no limit violations and no penetrations over 2 mm.

| Task | solve | query | render | fk | Confidence | Hardest part |
|---|---|---|---|---|---|---|
| T1 finger heart | 2 (+1 JSON parse error) | 6 | 5 | 0 | 70 | Getting the thumb/index crossing visible from the back of the hand. With the hand upright, the crossing sat behind the curled fist. I had to tilt the hand (fingers up-right, palm turned slightly) and use a less curled index MCP so the crossing sits above the knuckles. |
| T2 camera frame | 6 | 5 | 6 | 0 | 85 | (1) The thumb reaches only about 65° from the fingers (cmc_flex -30 is the limit), so the L was a parallelogram. I fixed it with index mcp_abd -15, thumb hyperextension and a tilted `fingers_toward`. (2) Tip to tip at a right-angle corner fails the "contact within 6 mm of the site" rule unless the thumb IP bends about 30°. The solver first chose IP 45° over moving the placement. I had to work out the right wrist position by hand from `query` fingertip positions. |
| T3 heart hands | 7 | 9 | 10 | 0 | 65 | Hand orientation. The thumb can point at most about 85° from the fingers (cmc_flex 35 + cmc_abd 75), so for the thumbs to point down, the whole hand must lean in toward the center. When `right.place` was in `vary`, the solver turned the hand back upright and the thumbs went flat. I locked the placement and only moved the wrist. Index tips that meet side by side (pointing down) fail both the `tip` and the `nail` goals on the 6 mm rule, so the goals are on segments (`index.distal ~ index.distal`, `middle.distal ~ middle.distal`). |
| T4 offering palm | 0 | 2 | 2 | 0 | 75 | No goals needed. I only had to keep the thumb from lifting straight up (cmc_abd lifts toward the palm side, which here is up). |
| T5 pointing | 5 | 4 | 5 | 0 | 75 | The aim worked first try. Getting the thumb to lie *across* the middle finger, rather than alongside the index like a finger gun, was hard. The solver keeps pulling cmc_flex back toward the start and straightens the IP to keep the pad within 6 mm. A second goal (thumb tip to ring) failed on the site-distance rule. |
| T6 OK sign | 2 | 2 | 4 | 0 | 80 | Making the circle visible with the palm facing the viewer. With the palm straight at +Z, the ring is edge-on from the front. My cross-product reasoning said to turn the palm toward -X, but that was wrong: +X was what opened the ring to the front camera. |
| T7 bunny ears | 2 | 2 | 2 | 0 | 85 | Small. A second goal (thumb.distal ~ pinky.middle) let the thumb cover both fingers, but the solver uncurled the pinky somewhat to get there. |

## What was confusing or missing

- **The 6 mm "contact near the site" rule is the main source of solver failures and odd bends.** Two fingertips meeting end to end at an angle (T2 corner), or side by side (T3), cannot satisfy `tip ~ tip` or `nail ~ nail` without bending joints. The miss message gives the distance but no hint. A per-goal `"within_mm"`, or a "tip region" site covering the whole fingertip cap, would help.
- **`vary` with `right.place` turns the hand freely.** Nothing lets me vary only the wrist position, or hold an orientation, for example `right.place.wrist` or a "keep thumb pointing toward X" aim. `aim` seems to work only for fingers; nothing says whether `aim` accepts the thumb.
- **The solver prefers bending joints over moving the placement**, even when the placement is in `vary`. The weighting between meters and degrees is not documented, so I could not predict which values it would change.
- **The thumb's reach is hard to picture from the Rz·Rx·Ry formula.** One sentence would have saved several iterations: the thumb is at most about 65° from the fingers in the palm plane and about 85° out of it, and to point it opposite the fingers you must turn the hand. Likewise for which side the OK/pinch ring opens to.
- `solve` rewrites the file with a different layout (it splits the fields onto separate lines). That broke my `sed` edits, which were written against the original layout. I switched to small Python edits.
- `query` printed the `checks`/`aim` headings twice in one run (T5).
- Nothing reports the angle between the thumb and the index (for "L at ~90°"). I judged it by eye from renders.

## What would have saved the most work

1. A way to tell the solver "move the placement, not the fingers": per-value weights, or vary `right.place.wrist` only. In T2 and T3 I worked out wrist positions by hand from fingertip coordinates.
2. A looser contact option for tip-to-tip / nail-to-nail goals (configurable site tolerance), so the natural pose would count as solved without forced joint bends.
3. A short "orientation cookbook" in the spec. Examples: palm toward viewer with the OK ring visible needs palm_facing tilted toward the thumb side; the thumb points down only if the hand leans. Most of my render calls went into finding orientations, not joint values.
