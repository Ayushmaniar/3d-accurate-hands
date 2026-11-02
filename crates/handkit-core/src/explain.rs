//! Text for `handkit explain`: the conventions and every parameter, from the same table the
//! limit checks use.

use crate::params::{self, PARAMS};

pub const CONVENTIONS: &str = "\
World: meters and degrees. Y up, X right, +Z toward the default front camera.
Rest pose (all values 0): forearm from the elbow straight up (+Y, 0.26 m) to the wrist; hand flat
and straight up, palm facing +Z, back of the hand (nails) facing -Z. Right hand: thumb on +X,
then index, middle, ring, pinky toward -X. Left hand: the exact mirror (thumb on -X).
Joint values mean the same for both hands (anatomical); the rig mirrors the left hand.

Placement, per hand, one of:
  place: { wrist, palm_facing, fingers_toward }   the hand frame directly: wrist position, the
      direction the palm faces and the direction the fingers point (at wrist_flex/dev 0 the
      straight fingers point exactly there). The forearm follows from twist and the wrist angles.
      The two directions need not be at right angles or unit length: fingers_toward is kept
      and palm_facing is made perpendicular to it.
      The right thumb is on the side of fingers_toward x palm_facing (cross product); the left
      thumb on the opposite side. Example: palm_facing [0,-1,0], fingers_toward [1,0,0] puts the
      right thumb toward -Z and the left thumb toward +Z.
      Add forearm_toward: [x, y, z] (the direction from the wrist toward the elbow) to aim the
      forearm: it sets wrist_flex and wrist_dev (any written are replaced), so the hand stays
      exactly where placed and the arm comes in from that side. `query` reports the wrist
      angles as limit violations if the bend is more than a wrist can do.
      To place from the picture, with the pose's camera: screen: [x, y] (fractions of the
      frame: x 0 left edge to 1 right, y 0 top to 1 bottom) and depth (meters from the camera
      along its view direction) put the wrist there, instead of wrist; palm_facing,
      fingers_toward and forearm_toward work as with wrist. elbow_screen: [x, y] instead of
      forearm_toward aims the forearm so the elbow lands on that point of the frame (the
      farther of the two points a forearm's length from the wrist; out of reach, the forearm
      points toward it), e.g. [1.05, 0.9] brings the arm in from past the right edge. The
      frame's shape is the pose's top-level \"size\": [width, height] (default 16:9).
  place: { elbow, forearm_rotation_deg: [rx, ry, rz] }   rotates the rest pose about the elbow,
      matrix Rx*Ry*Rz (rz applied first, about world axes). For a left hand it rotates the
      mirrored rest pose.
  A hand written as { \"mirror\": \"right\", ... } copies the right hand's joint values and takes the
  mirror image of its placement across the plane x = 0; any keys given override the copy.

Hands and people: \"hands\" holds right and left (one person) and, for more people,
<person>.right and <person>.left (person: lowercase letters, digits and _), e.g. \"ana.right\".
The name is the prefix everywhere a hand is named: sites (ana.right.index.tip), goals and vary
patterns (ana.left.*), checks, mirror (\"mirror\": \"ana.right\" on ana.left), `bones`. Every hand
is placed in the same world. A top-level \"people\": { \"ana\": { \"look\": { \"skin\": \"brown\" } } }
gives a person their own look (over the file's look); right and left belong to no person.
\"scale\": 0.75 on a person (0.5 to 1.5) sizes their hands and forearms (a child's): every length,
site, reach and mesh scales; angles do not. Grips between hands of two sizes still solve.

Chains: wrist = twist (about the forearm) -> wrist_dev -> wrist_flex. Fingers: knuckle spread
(rest fan + mcp_abd) -> mcp_roll -> mcp_flex -> pip_flex -> dip_flex. Thumb: base at (0.020, 0.018, 0.008)
in the hand frame (x toward the thumb, y toward the fingers, z out of the palm); rest
orientation = lean 35 deg toward the thumb side, lift 10 deg out of the palm, turn the pad 55
deg toward the index. Exactly, with rotations about the hand frame's axes applied left to right
(each about the already-rotated axes): Rz(-35 + cmc_flex) * Rx(10 + cmc_abd) *
Ry(-55 - cmc_rot). So positive cmc_flex undoes the lean, swinging the thumb toward the pinky
(at cmc_flex 35 it points along the fingers, before the lift); positive cmc_abd lifts it toward
+Z; positive cmc_rot turns the pad further toward the fingers. mcp_flex and ip_flex then rotate
about the thumb's own X axis, curling the tip toward the pad. At rest the right thumb points
along (0.57, 0.81, 0.17) and its pad faces (-0.73, 0.39, 0.57), toward the index and the viewer.
Run `handkit fk` on a pose to see every joint's position and axes.

Sizes: fingers (knuckle x, y; proximal, middle, distal length; radius): index (0.030, 0.093;
0.040, 0.025, 0.020; 9.5 mm), middle (0.010, 0.096; 0.045, 0.028, 0.021; 9.8 mm), ring
(-0.010, 0.091; 0.042, 0.027, 0.021; 9.2 mm), pinky (-0.029, 0.083; 0.033, 0.019, 0.018;
8.2 mm). Knuckles lie in the palm plane (z = 0). Each later segment is 10% thinner. Thumb
segments 0.046, 0.034, 0.027 m, radius 13.5, 10.5, 9.5 mm. Whole hand about 0.19 m.

Sites (surface points, named <hand>.<name>): for every digit tip (end of the fingertip), pad
(fingerprint), nail and knuckle (back of the MCP joint); for index to pinky side_radial and
side_ulnar (thumb side and pinky side of the fingertip) and dip_radial, dip_ulnar (sides of the
last joint); for the thumb side_palmar and side_dorsal (its two edges: the pad faces the index,
so one edge faces the palm side and one the back of the hand); for the hand palm_center,
palm_heel, wrist_back. Example: right.thumb.pad.
Where they sit, in each fingertip segment's frame (y along the segment from its base joint, z
toward the palm side, r its radius, l its length): tip (0, l + r, 0), pad (0, 0.55 l, r), nail
(0, 0.6 l, -r), side_radial (r, 0.5 l, 0), side_ulnar (-r, 0.5 l, 0); the thumb's side_palmar
and side_dorsal are the same two points as radial and ulnar. The palm surface is 12.5 mm in
front of the palm plane (palm_center (0, 0.055, 0.013) in the hand frame) and the back of the
hand 12.5 to 16 mm behind it.

Touching: a gap between surfaces of 0 to 3 mm. `query` lists contacts (across hands, anything
with a thumb, fingertips on the palm), penetrations (overlap over 2 mm), the nearest part to
each thumb tip, the surface gap for each pair in `checks` (a pair names sites or segments such
as right.index.middle; segments are metacarpal, proximal, middle, distal, and forearm), and
how well fingers aim at `probes`; `query --json` gives the same (with goals and fingertip
positions) as JSON.

Couplings (version 1 files): a dip_flex left out follows 2/3 of pip_flex.

Goals: \"goals\": [...] says what must be true; `handkit solve` changes values until it is and
writes them back, keeping changes small, joints inside their limits and fingers out of each
other (it may leave a penetration it cannot fix; it says so). If every goal already holds it
changes nothing; when a goal is still missed it writes the pose as far as it got and exits
with code 2. Note: `place` fixes the hand itself; wrist angles then swing the forearm, not
the hand.
  { \"touch\": [a, b] }   a and b are sites or segments. The surfaces meet (gap within 1 mm) and,
      for a site, the contact lands within 6 mm of it. Fingers do not roll about their own
      axis, so two pads seldom meet dead center: a contact a few mm toward the side of a pad
      is expected. Add \"gap_mm\": 15 to hold them that far apart instead (sites are then
      measured point to point, segments surface to surface).
  { \"aim\": \"right.index\", \"at\": [x, y, z] }   the finger's line (knuckle through tip) passes
      through the point, within 1 deg. The point is in world space, so an aim belongs to its
      placement: a hand-level \"from\" leaves a gesture's aims behind.
  { \"grip\": [a, b], \"style\": s }   two people's hands hold each other; a and b are whole hands
      (\"ana.right\"), a the leader, b the follower. Styles: handshake (two right or two left
      hands, palms together, thumbs over the backs, fingers wrapped round), interlaced (a right
      and a left side by side, fingers spread and alternating, each curled over the back of the
      other hand, both thumbs over the other's back), cupped (right with left: holding hands
      side by side, the leader's palm in the follower's, the follower's fingers curled round
      the leader's pinky edge onto its back and its thumb closed over the leader's index; two
      rights: a palm up cradles the other's fingers), high_five (palms flat, fingers up and
      spread), fist_bump (fists knuckles to knuckles), pinky (two rights in loose fists,
      thumbs resting on the index, both pinkies bent and hooked round each other).
      Unless the grip already holds, `solve` gives both hands the style's open shape, places
      the follower on the leader where the style puts it (only the follower placed: the
      leader moves to it; neither: the leader goes to a default spot near the origin), slides
      it in until the hands touch, then closes each gripping digit until it meets the other
      hand (or its own) and stops; the solver then tidies what is left. The result is plain
      joint values and placements, free to edit. It holds when every closing digit's last
      segment is within 1 mm of the other hand, or of the part the style lands it on (the
      pinkies on each other's middle segment, an interlaced fingertip on the back of the
      other hand; `query` lists each tip's gap in mm), the hands touch, and nothing overlaps
      more than 2 mm. A grip takes no touch or aim; its
      vary defaults to the closing digits and the follower's place.
  \"vary\": [...] on a goal chooses what may change (default: every joint of each digit a touch
      names, plus palm_cup for ring and pinky and the wrist for palm sites; for aim, the wrist
      and that finger's knuckle). Patterns: right.thumb.* (one digit), right.index.pip_flex
      (one value), right.*.mcp_abd (that joint on every digit), right.wrist (twist, wrist_flex,
      wrist_dev), right.palm_cup, right.place (the hand's placement; right.place.position
      only moves it, right.place.rotation only turns it), right.* (everything on the hand).
      Placement is never varied unless named. A hand that mirrors the other varies the
      other's values, so the pair stays a mirror.
  Shapes that read right: OK circle = thumb.pad ~ index.tip; flat pinch = thumb.pad ~
  index.pad; finger heart = thumb.pad ~ index.dip_radial. Start from roughly the right shape:
  the solver finds the nearest solution, not the most natural one.

Gestures: `handkit gestures` lists a library of finished poses; `handkit gesture <name>` prints
one. \"from\": \"<name>\" at the top of a file starts from a whole gesture (keys you write win;
its goals are added to yours). On a hand, { \"from\": \"ok\" } takes that gesture's joint
values for this hand (not its placement), and its goals renamed to this hand. handshake, holding_hands, interlaced_hands, high_five_pair,
fist_bump_pair and pinky_promise are two people's (ana's and ben's) hands held by a grip.

Motion: `handkit explain clips` (keyed poses over time, checked and previewed as a whole).
Looks: `handkit explain looks` (skin tones, toon, spiderverse, arcane, clay, motion blur).
Sign language: `handkit explain sign` (the ASL alphabet, `handkit spell`, sign clips).";

pub const CLIPS: &str = "\
Clips: motion as keyed poses. A clip file:
  { \"version\": 1, \"fps\": 30, \"duration\": 9.0, \"size\": [1920, 1080],
    \"camera\": { \"position\": [..], \"target\": [..], \"fov\": 35 },
    \"keys\": [ { \"t\": 0, \"hands\": {..}, \"goals\": [..] }, { \"t\": 1.2, \"move\": 0.4, ... }, ... ] }
Any number of hands (see Hands and people in `handkit explain`), with \"people\" as in a pose.
duration defaults to the last key + 0.5 s; size (the video's, for the framing check, `sheet`
and screen placements) to 16:9. A key's place.screen uses the clip's camera as of that key.

A key is a pose file plus timing. Every key starts from the previous key: write only what
changes (a key { \"t\": 2, \"hands\": { \"right\": { \"index\": { \"mcp_flex\": 0 } } } } straightens one
finger and keeps everything else). On a hand, \"from\": \"<gesture>\" takes that gesture's joint
values (keeping the hand where it is); \"from\" at the top of a key takes the whole gesture,
placement and goals included. A placement written replaces the previous one. Every hand must be
in the first key. Goals belong to their key only (repeat a goal in the next key to hold it).

Timing, per key (all optional except t):
  t          when the hand arrives at this key's pose (seconds).
  move       how long the move into it takes (default: the whole gap since the hand's last key).
             The hand holds the previous pose until t - move. Short = fast: 0.12 is a flick.
  ease       natural (default: smooth start and stop, a little follow-through), snappy (quick,
             overshoots and settles), smooth (no overshoot), linear.
  spring     { \"hz\": 4, \"damping\": 0.7 } tunes the settle (damping 0.2 bouncy to 1 none).
  anticipate a small move the other way first, as a fraction of the move (0.05 to 0.15).
  cascade    seconds between fingers in the move (default up to 0.035): closing runs pinky to
             index and then the thumb, opening frees the thumb, then index to pinky.
  pass       true makes the key a waypoint: the hand flows through it (passing near its pose)
             instead of slowing to a stop there. Use it for paths: an arc, a wave, a swing.
Per hand: every field but t can also go inside a hand of the key, timing that hand its own way
({ \"t\": 2, \"move\": 0.4, \"hands\": { \"right\": {..}, \"left\": { \"move\": 0.15, \"ease\": \"snappy\", .. } } });
what a hand leaves out comes from the key. A hand a key leaves out is not held by it: it keeps
moving between its own keys, so one hand can make a long move while the other makes several.
Moves overlap freely: a key can start moving before the previous one has arrived. A key at a
negative t starts the clip mid-move (the hand is already moving at 0). An ease or spring
written on a key applies to the move into that key only; spring overrides the ease's own.
Clip-wide: \"idle\": { \"amount\": 1, \"seed\": 0 } is a small breathing movement so a held hand
never freezes (0 turns it off); \"follow\": 1 scales how much relaxed fingers trail fast hand
moves.

Goals in clips: a goal held by two neighbouring keys (the same goal written in both) holds
through the move between them, every frame, so a pinch stays closed while the hand travels.
A goal only in the later key closes as the hand arrives; one only in the earlier key lets go
as the move starts. A hand's fingers never pass through each other or its palm: where a move
would, they slide past. A goal with only \"vary\" (no touch or aim) does the same in a pose
file: it keeps the parts it varies from passing through anything. Hands do not pass through
each other either: where two hands' moves would cross, the faster-moving one is pushed aside
(the slower barely moves; at equal speed the later-listed hand gives way), smoothly, and eases
back onto its keyed path once clear, on the side the keys put it (above stays above). Parts a
held goal joins (a touch between two hands) are not pushed apart. \"collide\": false turns
this off: clip-wide at the top, on a key (from its start on), or on one hand of a key. `check`
reports any overlap left with the keys either side; a deep keyed overlap is pushed far, so
route a big crossing yourself (a lifted waypoint, fingers opened first).

Grips in clips: keys in a row writing the same grip (hands and style) hold it. Only the leader
needs keying (the follower's place in later keys is ignored): the follower's hand rides on the
leader's, and its forearm follows through a critically damped spring (3 Hz, a few frames
behind), so a handshake pumped by one arm reads as led. In the first key with the grip the
hands arrive 0.3 s early and open, the gripping fingers closing over the last 0.3 s; the
follower blends onto the leader as it arrives. In the first key without it the fingers open
over 0.25 s, then the hands part. The pair does not collide with itself from the grip's
approach until it has parted. A grip in the first key holds from the start. Come in through a
pass waypoint a few cm short of the grip, palms facing and thumbs lifted toward their own
palms (open_palm with thumb cmc_abd 40, cmc_flex -25), so the thumbs pass each other; leave
the same way. high_five and fist_bump close no fingers: the hands simply meet at t.

Commands: `handkit check clip.json` (held goals, penetrations, joint speeds, pops, fingertips
leaving the camera's view, bare elbows in view; fix everything it reports; penetrations and pops
name the keys either side; \"at limit\" lines, joints held within 1 deg of a limit for over
0.3 s, are information: the pose asks a joint for more than it has), `handkit sheet clip.json
-n 12 --mesh` (a contact sheet at the clip's aspect; --at 1.2,1.3,1.4 for chosen times,
--background \"#2b2e33\" to match the video's), `handkit sample clip.json 1.5` (the pose
file at a time, to query or render; --wrist writes its placement as wrist, palm_facing and
fingers_toward), `handkit preview clip.json` (a quick MP4). To match a reference video:
`python tools/motion/compare.py ref.mp4 out.mp4 cmp` writes cmp.sheet.png (both at the same
times, in rows) and cmp.side.mp4 (side by side).

Where a site is on screen (e.g. a button behind the hand that reacts to a fingertip):
`handkit track clip.json --site right.index.tip [--site left.thumb.tip ...] [--fps 60]` prints
a JSON array, one frame per line: { \"t\", \"sites\": { \"<site>\": { \"screen\": [x, y], \"depth\",
\"world\": [x, y, z] } } }. screen is in fractions of the frame as place.screen (x 0 left to 1
right, y 0 top to 1 bottom) through the clip's camera and size, depth is meters along the
view (both null without a camera); fps defaults to the clip's. The same one call at a time:
clip.site(t, name) in the WASM build (JSON text), Clip::site_on_screen(t, name) in Rust.";

pub const LOOKS: &str = "\
Looks: how renderers draw the hands. A pose or a clip may carry a top-level \"look\":
  \"look\": { \"style\": \"toon\", \"skin\": \"tan\", \"nail\": \"#e9c4b8\", \"outline\": 3, \"motion_blur\": 4 }
(or just \"look\": \"clay\"). Every key is optional; left out, it takes the style's default.
  style        realistic (default: the baked skin maps in the chosen tone, soft key and cool
               rim light, screen-space subsurface scattering, warm shadows), toon (three cel
               bands, an ink outline and the main creases), spiderverse (toon with halftone dots
               in the shadows, a thick uneven line and misregistered magenta/cyan plates),
               arcane (painterly: soft warm/cool bands, brush strokes painted on the skin, fine
               dark lines), clay (matte modelling clay with lumps and fingerprints pressed in).
               Brush strokes, lumps, fingerprints and crease lines stay on the skin as it moves.
  skin         light, medium (default), tan, brown, dark, or \"#rrggbb\" (the albedo). Palms
               and nail beds stay lighter than the back of the hand, more so on darker skin.
  nail         a colour; default the skin, lighter and pinker. Nails come from the mesh.
  outline      ink line width in pixels at 1080p (scales with the frame height); defaults
               toon 3, spiderverse 4.5, arcane 1.6, realistic and clay 0 (none).
  motion_blur  sub-frame renders averaged per frame (1 = off; 3 to 6 for fast moves).
  shutter      shutter angle in degrees (default 180: half the frame time).
  halftone     spiderverse dot spacing in pixels at 1080p (default 8; 0 = no dots).
  misregister  spiderverse plate offset in pixels at 1080p (default 3; 0 = none).
  texture      0 to 1: brush strokes (arcane), lumps and fingerprints (clay); default 1.
  subsurface   0 to 1: how much scattered light realistic mixes in (default 0.35).
  arm          where the forearm ends (the rig stops at the elbow): \"bare\" (default: skin to
               a rounded end), \"fade\" (the forearm fades to transparent over its last 30%
               toward the elbow, ink and motion blur included), or { \"sleeve\": \"#rrggbb\",
               \"length\": 0.08 } (a rolled-up cloth sleeve, matte with soft folds and a rolled
               cuff, over the forearm from the elbow for length meters, 0.02 to 0.2; past the
               elbow it fades out as if it went on up the arm). With the elbow in view, use
               fade or a sleeve: `check` then lists it as information, not a problem.
               clips/demo_arms.json shows all three (docs/m6-arm.png).
The fframes player takes the same as flags over the file's look: --look toon,skin=tan,outline=4,
--look-json '{..}', --motion-blur 4, and --looks \"realistic;toon;arcane\" --look-every 2 to
cycle through looks (a demo). Each person's \"look\" (see \"people\") sets their skin, nail
colours and arm; the style is the file's. In Rust, handkit_fframes::look::Styler renders any
look (Styler::dress_as for a person's).";

pub const SIGN: &str = "\
Sign language (ASL): the manual alphabet is in the gesture library as asl_a ... asl_z (right
hand, palm to the viewer as a signer holds it; G and H point sideways toward the signer's left,
P and Q point down, C and O are turned so the viewer sees the opening; J and Z are their start
shapes), plus asl_ily (I-LOVE-YOU). They read from the viewer's side: the signer's left is +X.
  handkit spell HELLO -o hello.json   a clip that fingerspells a word (letters and spaces): the
      hand rises from rest, holds each letter briefly, bounces and slides for a double letter,
      traces J (the pinky hooks while the palm turns) and Z (the index draws it as the signer
      writes it, mirrored for the viewer), pauses between words and relaxes at the end. Big turns
      (into G, H and back) and the thumb tucking under the fingers (E, M, N, T) get a waypoint
      and a little more time. --rate 3.5 (letters per second; 3 to 4 is natural), --hold 0.4
      (the part of each letter held still), --look toon,skin=tan (the clip's look), --size
      1920x1080. In Rust: handkit_core::spell::spell, or spell_keys to add a word to your keys.
  Signs as clips (clips/asl_*.json): asl_hello (a salute from the forehead outward),
      asl_thank_you (flat hand from the chin forward and down), asl_i_love_you, asl_yes (the S
      hand nodding), asl_no (index and middle snapping to the thumb), asl_please (flat hand
      circling on the chest). The rig has no head or body: these are placed where the forehead,
      chin and chest would be (high and a little behind the hand's usual place, toward the
      signer's midline) and each description says where. docs/asl-alphabet.png shows every
      letter.";

pub fn all() -> String {
    let mut s = String::from(CONVENTIONS);
    s.push_str("\n\nParameters (key, range in degrees, meaning):\n");
    for p in PARAMS {
        s.push_str(&format!("  {} [{}, {}]: {}\n", p.key, p.min, p.max, p.doc));
    }
    s
}

pub fn one(key: &str) -> Result<String, String> {
    if key == "clips" || key == "clip" {
        return Ok(CLIPS.to_string());
    }
    if key == "looks" || key == "look" {
        return Ok(LOOKS.to_string());
    }
    if matches!(key, "sign" | "signs" | "asl" | "spell") {
        return Ok(SIGN.to_string());
    }
    let k = key.trim_start_matches("right.").trim_start_matches("left.");
    match params::find(k) {
        Some(p) => Ok(format!("{} [{}, {}]: {}", p.key, p.min, p.max, p.doc)),
        None => Err(format!(
            "unknown parameter \"{key}\". Known: {}",
            PARAMS.iter().map(|p| p.key).collect::<Vec<_>>().join(", ")
        )),
    }
}
