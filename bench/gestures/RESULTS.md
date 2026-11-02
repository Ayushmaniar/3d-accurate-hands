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

M1 gate ("blind runs beat the experiment on contacts"): met for Opus (6.5 to 1.1 mm), not for
Sonnet (10.3 to 23.9 mm). Visually, blind Opus's heart hands and OK sign read clearly and its
finger heart hides the thumb; blind Sonnet's finger heart, heart hands and OK sign are wrong.

What the runs asked for next: a contact solver ("touch A B by varying these joints"), or at
least per-joint sensitivity of a site (how far it moves per degree), because the thumb's
response to cmc changes is hard to predict once the hand is placed at an angle. The CLI run
used 71 query, 19 render and 38 fk calls; token use barely fell, so the M2 solver is where the
token budget will be won.

Sheets (reference photo next to each run): bench/gestures/sheets/.
