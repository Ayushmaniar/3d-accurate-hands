# m1-blind-opus: notes

All poses were reasoned by hand from POSE_SPEC.md, with no tool feedback. Working model I used:
hand frame x = thumb side, y = fingers, z = out of the palm. A finger at total flex angle a points
along (sin(fan)cos a, cos(fan)cos a, sin a). Thumb metacarpal direction is
(sin L cos A, cos L cos A, sin A), where L = 35 - cmc_flex and A = 10 + cmc_abd. The pad is
cos(t) n1 + sin(t) n2, where t = 55 + cmc_rot, n1 is the "lift" normal and n2 = (-cos L, sin L, 0).
I checked this against the rest vectors the spec gives, and both match. I assumed mcp_flex and ip_flex
rotate each later segment in the plane spanned by (current direction, pad).

| Task | Confidence | Hardest part |
|---|---|---|
| T1 finger heart | 40 | Getting "back of hand to viewer" and "heart points up" at the same time. They conflict, because the heart points out of the palm, so I tilted the palm 37 deg up. Thumb-to-index contact depends on the exact pad direction. |
| T2 camera frame | 50 | The rig cannot hold the thumb at 90 deg in the palm plane (cmc_flex min -20 gives 55 deg). I added mcp/ip hyperextension and index abd -10 to reach about 85 deg. Tip-to-tip corners were solved by symmetry, a 180 deg rotation about Z. |
| T3 heart hands | 30 | Choosing the hand orientation. I chose palms facing inward and down, so the flexion plane lies in the image plane, which puts the thumbs toward the camera. The thumb has to reach the midline at the same x as the index nail, which forced a 34 deg tilt. |
| T4 offering palm | 70 | Mostly matching the look: how much finger cascade, and the thumb's "relaxed" values. |
| T5 pointing | 60 | The aim is solved exactly in closed form. I used index mcp_abd -4 to cancel the rest fan and offset the line by the knuckle x = 0.030, which only holds if place's frame is exactly what I assumed. The thumb over the middle phalanx is a guess (gap about 4 mm by my estimate). |
| T6 OK sign | 45 | Thumb-tip to index-tip contact needs about 17 mm between axis ends. My estimate is about 3 mm between tip sites, but the error in my thumb model could easily be 1 cm. Seen from the palm, the circle is nearly edge-on. |
| T7 bunny ears | 50 | Thumb pad reaching across to the ring and pinky middle phalanges without hitting the extended middle finger. |

## Spec ambiguities / guesses

- **Order of thumb lean vs lift.** I assumed lean is applied first, then lift (it matches the rest vectors). It is not stated how mcp_flex/ip_flex axes relate to the pad: is the curl exactly toward the pad normal?
- **cmc_flex sign.** The spec says it "adds to the lean (toward the pinky)". I read it as effective lean toward the thumb = 35 - cmc_flex, but "adds to the lean" first reads as 35 + cmc_flex.
- **Site definitions.** `tip`, `pad`, `nail` and the `dip_*` sites have no stated offsets, so I guessed tip = axis end + radius along the bone and pad = middle of the distal segment. The radius of the thumb tip is ambiguous: 9.5 mm is listed for the distal segment, but fingers get "10% thinner" per segment.
- **Palm thickness.** No palm-surface z is given, so I could not check curled fingertips against the palm.
- **Thumb base position.** The CMC base is at y = 0.018, which I took as the metacarpal start, and the "0.046" segment as the metacarpal. That makes the thumb reach shorter than I'd expect, and it drove most of my contact geometry.
- **palm_cup.** "Turns cup degrees toward the thumb": it is unclear whether this turns the pinky's flexion plane (fan) or rolls it. I ignored it in the geometry.
- **Viewer for "seen from behind".** "Photographer's view" plus the photo convinced me the camera is at +Z with the hands in front of it. I am not sure that matches the grader's notion of "behind the hands".
- **Rest fan for straight lines.** For T5 I assumed the index's 4 deg rest fan is about the knuckle within the palm plane and that mcp_abd -4 makes it exactly parallel to fingers_toward.
- **mirror plus place override.** For T2 I wrote the left hand out in full instead of relying on `"mirror"` plus overriding `place`, because it is unclear whether an override replaces the whole place block.
