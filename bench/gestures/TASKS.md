# Gesture tasks

Write one pose file per task, `T1.json` … `T7.json`, following `docs/POSE_SPEC.md`. Reference photos are
in `experiments/joint-angles/refs/`; match the gesture and hand shape, not the exact framing. Where a task says "seen
from", add a `camera` that shows it that way.

1. **T1 finger heart** (`refs/9.png`). Right hand only, raised, back of the hand toward the
   viewer. Thumb and index cross near their tips to make the small Korean "finger heart":
   the thumb pad rests on the side of the index finger near its middle/tip joint, thumb tip
   just past it. Middle, ring and pinky curled into the palm.
2. **T2 camera frame** (`refs/1.png`). Both hands make an L (index straight, thumb out at
   ~90°, other fingers curled) and join into a rectangle the viewer looks through, seen from
   behind the hands (the photographer's view: camera near the face, looking past the hands).
   Left hand on top with its index pointing right and thumb pointing down; right hand below
   with its index pointing left and thumb pointing up. The left index tip touches the right
   thumb tip, and the left thumb tip touches the right index tip.
3. **T3 heart hands** (`refs/7.png`). Both hands, seen from behind (the photographer's view).
   Index to pinky of each hand curl over together to form the two top lobes of a heart, their
   fingertips/nails touching the other hand's at the top center; the thumbs point down and
   their tips touch at the bottom point of the heart.
4. **T4 offering palm** (`refs/6.png`). Right hand reaching toward the viewer and slightly down,
   palm up, fingers relaxed and slightly curled, thumb relaxed out to the side.
5. **T5 pointing**. Right hand, wrist at [0.15, -0.09, 0]. Point the index finger at the world
   point [-0.6, 0.3, -0.4]: the index finger's straight line must pass through it. Middle, ring,
   pinky curled into a fist; thumb resting over the curled middle finger.
6. **T6 OK sign**. Right hand, palm facing the viewer. Thumb tip and index tip touch to make a
   round circle; middle, ring and pinky extended and slightly fanned.
7. **T7 bunny ears**. Right hand, back of the hand toward the viewer, index and middle straight
   up and spread in a V, thumb holding the curled ring and pinky down.
