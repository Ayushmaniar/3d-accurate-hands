# Gesture benchmark results

Seven gestures (bench/gestures/TASKS.md), scored by `python bench/gestures/score.py`.
Contact error = |surface gap| in mm between the segments that must touch (T1, T2, T3, T6).
Penetrations = segment pairs overlapping by more than 2 mm, over all 7 poses. Every run
aimed the T5 index within 0.1 deg of the target, and no run broke a joint limit.

| Run | Interface | Contact error mean / max (mm) | Penetrations | Tokens per gesture (approx.) |
| --- | --- | --- | --- | --- |
| experiment blind-sonnet | capsule rig, spec only | 10.3 / 22.0 | 42 | 18k |
| experiment blind-opus | capsule rig, spec only | 6.5 / 10.8 | 16 | 22k |
| experiment loop-opus | capsule rig + renders | 1.6 / 4.2 | 14 | 28k |
| m1-blind-sonnet | M1 spec only | 23.9 / 51.0 | 50 | 15k |
| m1-blind-opus | M1 spec only | 1.1 / 3.6 | 15 | 23k |
| m1-cli-opus | M1 spec + handkit query/render/fk | 0.6 / 1.8 | 0 | 26k |
| m2-cli-sonnet | M2 spec + solve (no gesture library) | 0.2 / 0.7 | 9 | 18k |
| m2-cli-opus | M2 spec + solve (no gesture library) | 0.1 / 0.5 | 0 | 26k |

M1 gate ("blind runs beat the experiment on contacts"): met for Opus (6.5 to 1.1 mm), not for
Sonnet (10.3 to 23.9 mm). Visually, blind Opus's heart hands and OK sign read clearly and its
finger heart hides the thumb; blind Sonnet's finger heart, heart hands and OK sign are wrong.

What the runs asked for next: a contact solver ("touch A B by varying these joints"), or at
least per-joint sensitivity of a site (how far it moves per degree), because the thumb's
response to cmc changes is hard to predict once the hand is placed at an angle. The CLI run
used 71 query, 19 render and 38 fk calls; token use barely fell, so the M2 solver is where the
token budget will be won.

Sheets (reference photo next to each run): bench/gestures/sheets/.

## M2 (contact solver)

Both M2 runs wrote `goals` and ran `handkit solve` instead of searching angles by hand; neither
could use the gesture library (it contains several of these gestures). Every contact is now
within 0.7 mm. Sonnet improved the most: from 23.9 mm mean contact error blind to 0.2 mm, but
it left 9 penetrations (curled pinky into its metacarpal, and the heart hands' fingers into
each other) that it did not fix. Opus is clean on every measure, and its heart hands now show
two lobes and a point.

Token use did not fall (Opus 26k per gesture, as in M1; Sonnet 18k). The work moved from
joint angles to placement: most renders went into finding hand orientations and camera
framing ("seen from behind", which way the OK ring opens), and into letting the solver move
the hand without turning it (`right.place.position` was added after these runs). The cheap
path for a known gesture is now the library: `"from": "ok"` costs one read.

What the runs asked for next: an orientation cookbook in the spec (palm and finger directions
for common views), named camera views, a contact rule that suits fingertips meeting side by
side or at an angle, and per-value weights in `vary`.

Rig limits found while building the 44-gesture library (crates/handkit-core/gestures, sheet in
docs/gesture-library.png):
- The thumb cannot oppose its pad to the ring or pinky pad: lifting it out of the palm swings
  it forward, not across, so those gestures touch the fingertip instead. Needs a saddle-joint
  thumb model with coupled pronation (M3 rig work). `mcp_roll` (added in M2) fixed the middle.
- Neighbouring fingers are round capsules that already nearly touch at the knuckles, so
  fingers cannot squeeze together (tulip, tripod, crossed fingers read loosely).
- The thumb opens only about 65 deg from the index in the palm plane; a true 90 deg "L" is out
  of reach (camera frame becomes a parallelogram).

