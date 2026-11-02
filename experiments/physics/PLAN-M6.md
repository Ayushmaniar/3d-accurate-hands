# M6 plan: many hands, people, holding hands

The user's ask for M6: hand-holding, multiple hands, multiple people. M4's gate showed the gap
too: when one hand crosses another mid-move, nothing keeps them apart (the Burrows clip has 14
crossings that `check` now reports but the clip passes straight through).

## 1. More than two hands

A pose's `hands` keeps `right` and `left` (the default person) and also accepts
`<person>.right` / `<person>.left`, e.g. `"ana.right"`, `"ben.left"`. Everything that names a
hand or a site takes the same prefix: sites `ana.right.index.tip`, goals, `mirror`, `from`,
clips' per-hand timing, `check` lines, bones for the adapters (`sample --bones`), fframes and
three.js adapters (one model instance per hand). Optional `people` metadata per person: `look`
(skin tone etc.), `scale` (child or adult hand), `shoulder` (where that person's arms come
from, so placements can be checked for reach).

Internally a hand is identified by (person, side) instead of side alone.

## 2. Hands that don't pass through each other while moving

Soft collision between hands during clip sampling: per frame, the penetration between hand A
and every other hand (capsule sets, already in `query`) gives a depth and a contact normal;
the hand that is moving more is pushed along the normal by a smooth function of the depth
(continuous in time: no flips, no pops), and its fingers flex away a little. A key may opt
out (`"collide": false`) for deliberate overlap. `check` keeps reporting what is left.

## 3. Holding hands

- `grip` goals, solved like other goals: `{"grip": ["ana.right", "ben.left"], "style":
  "handshake" | "interlaced" | "cupped" | "pinky" | "fist_bump"}`. They place the two hands
  palm to palm (or as the style needs) and close the fingers around the other hand until the
  pads touch its surface, without penetration.
- `attach`: while a grip holds in a clip, the hands move together: the follower's placement is
  the leader's frame times the grip offset, through a damped spring (so a pull leads and the
  other hand trails a few frames, as real linked arms do).
- Library gestures for two people: handshake, holding hands walking (side by side), interlaced
  fingers, high five, fist bump, pinky promise, a hand on a shoulder is out of scope (no body).

## 4. Gate

An agent (fresh, minimal prompt) makes: two people shaking hands; a couple's interlaced hands
swinging; a high five; four hands stacking ("team hands-in"). `check` clean, no visible
passing-through, motion reads as natural. Plus re-check the Burrows clip with collisions on.
