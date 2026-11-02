# m2-cli-sonnet notes

Call counts are approximate (I did not log them exactly). No fk calls and no `explain` calls were used.

| Task | solve | query | render | fk | Confidence | Hardest part |
|---|---|---|---|---|---|---|
| T1 finger heart | 3 | 3 | 3 | 0 | 40 | The solver curls the index a lot to satisfy `thumb.pad ~ index.dip_radial`. The thumb sits behind the fist from the back view, so it hardly reads. |
| T2 camera frame | 3 | 4 | 2 | 0 | 55 | Reaching a true 90 degree L. The thumb limit (cmc_flex -30) gives about 65 degrees from the finger axis, so the frame is a slanted parallelogram. The placement also had to be tuned by hand from tip positions, because default `vary` does not move `place`. |
| T3 heart hands | 4 | 3 | 5 | 0 | 40 | Thumbs have to point down, which forces the fingers about 65 degrees off vertical. The result is a flat diamond, not a heart with two lobes. When `place` was allowed to vary, the solver spun the hands toward the camera. |
| T4 offering palm | 0 | 1 | 3 | 0 | 65 | Only picking the placement vectors and the camera. There are no goals, so no solving was needed. |
| T5 pointing | 3 | 3 | 3 | 0 | 75 | A 4 mm pinky distal vs metacarpal penetration in the fist remains. palm_cup trades it for finger-finger penetrations. |
| T6 OK sign | 1 | 1 | 1 | 0 | 75 | Easy. Touching tips produce a small circle. |
| T7 bunny ears | 1 | 1 | 1 | 0 | 70 | Easy. The solver raised palm_cup to 21 to get the thumb over the ring finger. |

## Confusing or missing

- A "back of hand toward viewer" or "palm facing up" pose needs the cross-product thumb rule worked out by hand each time. A query line such as "thumb points to world +X" would save that.
- Default `vary` does not include `place`, and `solve` reports "0 iterations" with no hint when a goal is too far for the joints to reach. `query` should say which side to move, for example "left index tip is 28 mm from right thumb tip, move by this vector".
- For two-hand poses, `place` is the only way to position the hands, and `vary` on place moves the hands arbitrarily (T3 ended up facing the camera). A "translate only" vary would help.
- The thumb range limits the angle between thumb and index to roughly 65 to 76 degrees. A true 90 degree L (T2) is impossible without tilting the whole hand frame.
- Whether `"mirror": "left"` is legal on a right hand is not stated. I kept the left hand as the mirror of the right.
- The hero camera defaults are not helpful. I had to guess camera distances. A "frame all" camera option would help.
- `vary` pattern semantics for mirrored hands ("varies the other's values") caused the solver to change `right.*` values when I asked for left goals.

## What would save the most work

1. Gesture templates would obviously help, but they were forbidden here.
2. An `auto-place` option, or a hint in `query` of the vector between goal points, so initial placement does not have to be tuned by trial.
3. A non-fixed-roll option for the contact goals (the spec says fingers do not roll).
4. A view of the pose from an arbitrary named direction ("from_behind") rather than only fixed views.
