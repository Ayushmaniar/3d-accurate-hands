//! Grips: two hands holding each other, `{"grip": [leader, follower], "style": ..}`.
//!
//! A style says where the follower's hand goes in the leader's hand frame, the open shape both
//! hands start from and which digits close around the other hand. Solving a grip that does not
//! hold (`prepare`) works like taking a hand: the follower is placed by the style a little way
//! off and slides in until the hands touch, then every gripping digit of both hands curls (all
//! its joints together) and each joint stops where the segment it moves meets a surface (the
//! other hand or its own), until the fingertip lands. Nothing passes through anything, and the
//! result is ordinary joint values and placements in the file. Clips move a held grip's follower
//! with its leader (`clip`).

use glam::{DAffine3, DMat3, DVec3};
use serde_json::{Value, json};

use crate::Solved;
use crate::goals::{GoalStatus, TOUCH_TOL_MM};
use crate::params;
use crate::pose::{Goal, HandId, Side};
use crate::query::{PENETRATION_MM, seg_dist};
use crate::rig::{Capsule, Digit, HandGeom, Seg};

/// One way two hands hold each other: where the follower goes and how both hands close.
struct Hold {
    /// The follower's hand frame in a right leader's frame (x toward the thumb, y along the
    /// fingers, z out of the palm): its wrist, the directions its palm faces and its fingers
    /// point, and the direction from its wrist toward its elbow.
    wrist: [f64; 3],
    palm: [f64; 3],
    fingers: [f64; 3],
    forearm: [f64; 3],
    /// The direction, in the leader's frame, the follower comes in along (it starts 6 cm back
    /// and slides until the hands touch); none places it exactly.
    approach: Option<[f64; 3]>,
    /// Per hand (leader, follower): the shape before closing, and the digits that close.
    shape: [Shape; 2],
    close: [&'static [Digit]; 2],
    /// Contacts the grip makes, each like a touch goal: (hand, site or segment, hand, site or
    /// segment) with 0 the leader and 1 the follower, e.g. (0, "pinky.distal", 1, "pinky.middle").
    /// A closing digit named here lands where this says instead of anywhere on the other hand.
    lands: &'static [Land],
    /// Where the leader goes when neither hand is placed: wrist, palm_facing, fingers_toward,
    /// forearm_toward (a right leader; a left one is mirrored in x).
    home: [[f64; 3]; 4],
}

/// Joint values a hand starts from before its digits close.
type Shape = &'static [(&'static str, f64)];

type Land = (usize, &'static str, usize, &'static str);

struct Style {
    name: &'static str,
    /// The hold when the follower is the same side as the leader (right with right), and the
    /// other.
    same: Option<Hold>,
    other: Option<Hold>,
}

const ALL: &[Digit] = &[Digit::Thumb, Digit::Index, Digit::Middle, Digit::Ring, Digit::Pinky];
const FINGERS: &[Digit] = &[Digit::Index, Digit::Middle, Digit::Ring, Digit::Pinky];
const THUMB_PINKY: &[Digit] = &[Digit::Thumb, Digit::Pinky];

/// A hand open to take another: fingers nearly straight, the thumb out of the way.
/// OPEN with a resting thumb, alongside the index: a hand lying draped across another.
const DRAPED: Shape = &[
    ("palm_cup", 6.0),
    ("thumb.cmc_flex", 5.0), ("thumb.cmc_abd", 12.0), ("thumb.cmc_rot", 10.0), ("thumb.mcp_flex", 10.0), ("thumb.ip_flex", 12.0),
    ("index.mcp_flex", 0.0), ("index.pip_flex", 4.0), ("index.dip_flex", 2.0), ("index.mcp_abd", 0.0), ("index.mcp_roll", 0.0),
    ("middle.mcp_flex", 0.0), ("middle.pip_flex", 4.0), ("middle.dip_flex", 2.0), ("middle.mcp_abd", 0.0), ("middle.mcp_roll", 0.0),
    ("ring.mcp_flex", 0.0), ("ring.pip_flex", 4.0), ("ring.dip_flex", 2.0), ("ring.mcp_abd", 0.0), ("ring.mcp_roll", 0.0),
    ("pinky.mcp_flex", 0.0), ("pinky.pip_flex", 4.0), ("pinky.dip_flex", 2.0), ("pinky.mcp_abd", 0.0), ("pinky.mcp_roll", 0.0),
];

const OPEN: Shape = &[
    ("palm_cup", 6.0),
    ("thumb.cmc_flex", -25.0), ("thumb.cmc_abd", -10.0), ("thumb.cmc_rot", 0.0), ("thumb.mcp_flex", 0.0), ("thumb.ip_flex", 0.0),
    ("index.mcp_flex", 0.0), ("index.pip_flex", 4.0), ("index.dip_flex", 2.0), ("index.mcp_abd", 0.0), ("index.mcp_roll", 0.0),
    ("middle.mcp_flex", 0.0), ("middle.pip_flex", 4.0), ("middle.dip_flex", 2.0), ("middle.mcp_abd", 0.0), ("middle.mcp_roll", 0.0),
    ("ring.mcp_flex", 0.0), ("ring.pip_flex", 4.0), ("ring.dip_flex", 2.0), ("ring.mcp_abd", 0.0), ("ring.mcp_roll", 0.0),
    ("pinky.mcp_flex", 0.0), ("pinky.pip_flex", 4.0), ("pinky.dip_flex", 2.0), ("pinky.mcp_abd", 0.0), ("pinky.mcp_roll", 0.0),
];

/// Open for a handshake: the thumb lifted toward the palm side, to pass over the top of the
/// other hand onto its back.
const SHAKE: Shape = &[
    ("palm_cup", 6.0),
    ("thumb.cmc_flex", -12.0), ("thumb.cmc_abd", 5.0), ("thumb.cmc_rot", 10.0), ("thumb.mcp_flex", 5.0), ("thumb.ip_flex", 5.0),
    ("index.mcp_flex", 0.0), ("index.pip_flex", 4.0), ("index.dip_flex", 2.0), ("index.mcp_abd", 0.0), ("index.mcp_roll", 0.0),
    ("middle.mcp_flex", 0.0), ("middle.pip_flex", 4.0), ("middle.dip_flex", 2.0), ("middle.mcp_abd", 0.0), ("middle.mcp_roll", 0.0),
    ("ring.mcp_flex", 0.0), ("ring.pip_flex", 4.0), ("ring.dip_flex", 2.0), ("ring.mcp_abd", 0.0), ("ring.mcp_roll", 0.0),
    ("pinky.mcp_flex", 0.0), ("pinky.pip_flex", 4.0), ("pinky.dip_flex", 2.0), ("pinky.mcp_abd", 0.0), ("pinky.mcp_roll", 0.0),
];

/// Fingers spread wide and straight, so the other hand's fingers fit between them: the eight
/// fingers of the two hands fan evenly from 29 deg toward the thumbs to -44 deg, alternating,
/// the follower's index outermost (with the follower 1 cm toward the thumbs).
const SPREAD_LEADER: Shape = &[
    ("palm_cup", 0.0),
    ("thumb.cmc_flex", -25.0), ("thumb.cmc_abd", -10.0), ("thumb.cmc_rot", 0.0), ("thumb.mcp_flex", 0.0), ("thumb.ip_flex", 0.0),
    ("index.mcp_flex", 30.0), ("index.pip_flex", 40.0), ("index.dip_flex", 24.0), ("index.mcp_abd", 14.6), ("index.mcp_roll", 0.0),
    ("middle.mcp_flex", 30.0), ("middle.pip_flex", 40.0), ("middle.dip_flex", 24.0), ("middle.mcp_abd", -2.2), ("middle.mcp_roll", 0.0),
    ("ring.mcp_flex", 30.0), ("ring.pip_flex", 40.0), ("ring.dip_flex", 24.0), ("ring.mcp_abd", 19.0), ("ring.mcp_roll", 0.0),
    ("pinky.mcp_flex", 30.0), ("pinky.pip_flex", 40.0), ("pinky.dip_flex", 24.0), ("pinky.mcp_abd", 34.8), ("pinky.mcp_roll", 0.0),
];
const SPREAD_FOLLOWER: Shape = &[
    ("palm_cup", 0.0),
    ("thumb.cmc_flex", -25.0), ("thumb.cmc_abd", -10.0), ("thumb.cmc_rot", 0.0), ("thumb.mcp_flex", 0.0), ("thumb.ip_flex", 0.0),
    ("index.mcp_flex", 30.0), ("index.pip_flex", 40.0), ("index.dip_flex", 24.0), ("index.mcp_abd", 25.0), ("index.mcp_roll", 0.0),
    ("middle.mcp_flex", 30.0), ("middle.pip_flex", 40.0), ("middle.dip_flex", 24.0), ("middle.mcp_abd", 8.2), ("middle.mcp_roll", 0.0),
    ("ring.mcp_flex", 30.0), ("ring.pip_flex", 40.0), ("ring.dip_flex", 24.0), ("ring.mcp_abd", 8.6), ("ring.mcp_roll", 0.0),
    ("pinky.mcp_flex", 30.0), ("pinky.pip_flex", 40.0), ("pinky.dip_flex", 24.0), ("pinky.mcp_abd", 24.4), ("pinky.mcp_roll", 0.0),
];

/// Flat and spread, the fingers a little back: a high five.
const SLAP: Shape = &[
    ("palm_cup", 0.0),
    ("thumb.cmc_flex", -15.0), ("thumb.cmc_abd", -5.0), ("thumb.cmc_rot", 0.0), ("thumb.mcp_flex", 0.0), ("thumb.ip_flex", 3.0),
    ("index.mcp_flex", -6.0), ("index.pip_flex", 4.0), ("index.dip_flex", 2.0), ("index.mcp_abd", 10.0), ("index.mcp_roll", 0.0),
    ("middle.mcp_flex", -6.0), ("middle.pip_flex", 4.0), ("middle.dip_flex", 2.0), ("middle.mcp_abd", 0.0), ("middle.mcp_roll", 0.0),
    ("ring.mcp_flex", -6.0), ("ring.pip_flex", 4.0), ("ring.dip_flex", 2.0), ("ring.mcp_abd", 8.0), ("ring.mcp_roll", 0.0),
    ("pinky.mcp_flex", -6.0), ("pinky.pip_flex", 4.0), ("pinky.dip_flex", 2.0), ("pinky.mcp_abd", 15.0), ("pinky.mcp_roll", 0.0),
];

const FIST: Shape = &[
    ("palm_cup", 10.0),
    ("thumb.cmc_abd", 44.1), ("thumb.cmc_flex", 2.8), ("thumb.cmc_rot", 48.8), ("thumb.ip_flex", 24.0), ("thumb.mcp_flex", 48.7),
    ("index.mcp_abd", 0.0), ("index.mcp_flex", 85.0), ("index.pip_flex", 95.0), ("index.dip_flex", 63.0), ("index.mcp_roll", 0.0),
    ("middle.mcp_abd", 0.0), ("middle.mcp_flex", 88.0), ("middle.pip_flex", 98.0), ("middle.dip_flex", 65.0), ("middle.mcp_roll", 0.0),
    ("ring.mcp_abd", -3.0), ("ring.mcp_flex", 88.0), ("ring.pip_flex", 98.0), ("ring.dip_flex", 65.0), ("ring.mcp_roll", 0.0),
    ("pinky.mcp_abd", 0.0), ("pinky.mcp_flex", 85.0), ("pinky.pip_flex", 88.0), ("pinky.dip_flex", 58.0), ("pinky.mcp_roll", 0.0),
];

/// A loose fist, the thumb over the index, the pinky out and bent to hook.
const PINKY_HOOK: Shape = &[
    ("palm_cup", 8.0),
    ("thumb.cmc_flex", 3.0), ("thumb.cmc_abd", 45.0), ("thumb.cmc_rot", 48.0), ("thumb.mcp_flex", 30.0), ("thumb.ip_flex", 10.0),
    ("index.mcp_abd", 0.0), ("index.mcp_flex", 80.0), ("index.pip_flex", 95.0), ("index.dip_flex", 58.0), ("index.mcp_roll", 0.0),
    ("middle.mcp_abd", 0.0), ("middle.mcp_flex", 83.0), ("middle.pip_flex", 97.0), ("middle.dip_flex", 60.0), ("middle.mcp_roll", 0.0),
    ("ring.mcp_abd", 0.0), ("ring.mcp_flex", 85.0), ("ring.pip_flex", 98.0), ("ring.dip_flex", 60.0), ("ring.mcp_roll", 0.0),
    ("pinky.mcp_abd", 0.0), ("pinky.mcp_flex", 10.0), ("pinky.pip_flex", 80.0), ("pinky.dip_flex", 45.0), ("pinky.mcp_roll", 0.0),
];

/// Holding hands side by side: the inner hand's fingers a little bent, its thumb lifted to come
/// over the other hand; the outer hand's fingers curved to wrap round the inner hand's edge.
const CUP_INNER: Shape = &[
    ("palm_cup", 6.0),
    ("thumb.cmc_flex", -30.0), ("thumb.cmc_abd", -5.0), ("thumb.cmc_rot", 0.0), ("thumb.mcp_flex", 0.0), ("thumb.ip_flex", 0.0),
    ("index.mcp_flex", 15.0), ("index.pip_flex", 20.0), ("index.dip_flex", 10.0), ("index.mcp_abd", 0.0), ("index.mcp_roll", 0.0),
    ("middle.mcp_flex", 15.0), ("middle.pip_flex", 20.0), ("middle.dip_flex", 10.0), ("middle.mcp_abd", 0.0), ("middle.mcp_roll", 0.0),
    ("ring.mcp_flex", 15.0), ("ring.pip_flex", 20.0), ("ring.dip_flex", 10.0), ("ring.mcp_abd", 0.0), ("ring.mcp_roll", 0.0),
    ("pinky.mcp_flex", 15.0), ("pinky.pip_flex", 20.0), ("pinky.dip_flex", 10.0), ("pinky.mcp_abd", 0.0), ("pinky.mcp_roll", 0.0),
];
const CUP_OUTER: Shape = &[
    ("palm_cup", 10.0),
    ("thumb.cmc_flex", -5.0), ("thumb.cmc_abd", 25.0), ("thumb.cmc_rot", 15.0), ("thumb.mcp_flex", 15.0), ("thumb.ip_flex", 15.0),
    ("index.mcp_flex", 10.0), ("index.pip_flex", 40.0), ("index.dip_flex", 20.0), ("index.mcp_abd", 0.0), ("index.mcp_roll", 0.0),
    ("middle.mcp_flex", 10.0), ("middle.pip_flex", 40.0), ("middle.dip_flex", 20.0), ("middle.mcp_abd", 0.0), ("middle.mcp_roll", 0.0),
    ("ring.mcp_flex", 10.0), ("ring.pip_flex", 40.0), ("ring.dip_flex", 20.0), ("ring.mcp_abd", 0.0), ("ring.mcp_roll", 0.0),
    ("pinky.mcp_flex", 10.0), ("pinky.pip_flex", 40.0), ("pinky.dip_flex", 20.0), ("pinky.mcp_abd", 0.0), ("pinky.mcp_roll", 0.0),
];

const STYLES: &[Style] = &[
    // Right with right, palms together and the hands crossed, webs locked: the fingers wrap
    // the other's pinky edge, the thumb lies over the back of the other's hand.
    Style {
        name: "handshake",
        same: Some(Hold {
            wrist: [0.044, 0.15, 0.03], palm: [0.0, 0.0, -1.0], fingers: [-0.643, -0.766, 0.0], forearm: [0.643, 0.766, 0.25],
            approach: Some([0.0, 0.0, 1.0]),
            shape: [SHAKE, SHAKE],
            close: [ALL, ALL],
            lands: &[],
            home: [[-0.075, 0.0, 0.0], [0.0, 0.0, -1.0], [1.0, -0.4, 0.0], [-1.0, 0.0, 0.25]],
        }),
        other: None,
    },
    // A right and a left hand side by side, palms together and fingers pointing the same way,
    // every finger between two of the other's, curled over the back of the other's hand.
    Style {
        name: "interlaced",
        same: None,
        other: Some(Hold {
            wrist: [0.01, 0.0, 0.045], palm: [0.0, 0.0, -1.0], fingers: [0.0, 1.0, 0.0], forearm: [0.0, -1.0, 0.6],
            approach: None,
            shape: [SPREAD_LEADER, SPREAD_FOLLOWER],
            close: [ALL, ALL],
            lands: &[
                (0, "index.pad", 1, "middle.metacarpal"), (0, "middle.pad", 1, "ring.metacarpal"), (0, "ring.pad", 1, "pinky.metacarpal"), (0, "pinky.pad", 1, "pinky.metacarpal"),
                (1, "index.pad", 0, "index.metacarpal"), (1, "middle.pad", 0, "middle.metacarpal"), (1, "ring.pad", 0, "ring.metacarpal"), (1, "pinky.pad", 0, "pinky.metacarpal"),
                (0, "thumb.pad", 1, "index.metacarpal"), (1, "thumb.pad", 0, "index.metacarpal"),
            ],
            home: HANG,
        }),
    },
    // One hand wraps the other without interlacing. Right with right: the first, palm up,
    // cradles the second's fingers and closes over them, its thumb on the back of the second's
    // knuckles; the second lies palm down across it, fingers draped, its thumb resting by the index. Right with left: holding
    // hands side by side, palm in palm, the second's fingers crossing the first's at 60 deg
    // toward its pinky edge and curled round that edge onto its back, the second's thumb over
    // the back of the first's index; the first's fingers close round the second's index edge.
    Style {
        name: "cupped",
        same: Some(Hold {
            wrist: [0.14, 0.046, 0.06], palm: [0.0, 0.0, -1.0], fingers: [-0.95, 0.15, -0.15], forearm: [0.95, -0.15, 0.15],
            approach: Some([0.0, 0.0, 1.0]),
            shape: [OPEN, DRAPED],
            close: [ALL, FINGERS],
            lands: &[],
            home: [[-0.08, 0.0, 0.0], [0.0, 1.0, 0.0], [0.6, 0.0, -0.8], [-0.6, -0.1, 0.8]],
        }),
        other: Some(Hold {
            wrist: [0.09, 0.02, 0.038], palm: [0.0, 0.0, -1.0], fingers: [-0.9397, 0.342, 0.0], forearm: [0.6, -0.6, 0.55],
            approach: None,
            shape: [CUP_INNER, CUP_OUTER],
            close: [ALL, ALL],
            lands: &[(1, "thumb.pad", 0, "index.metacarpal")],
            home: HANG,
        }),
    },
    // Palms slap flat, fingers up and spread: right with right (thumbs on opposite sides) or
    // right with left (thumbs on the same side).
    Style {
        name: "high_five",
        same: Some(Hold {
            wrist: [0.0, 0.0, 0.03], palm: [0.0, 0.0, -1.0], fingers: [0.0, 1.0, 0.0], forearm: [0.0, -1.0, 0.5],
            approach: Some([0.0, 0.0, 1.0]),
            shape: [SLAP, SLAP],
            close: [&[], &[]],
            lands: &[],
            home: [[-0.013, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [-0.5, -1.0, 0.0]],
        }),
        other: Some(Hold {
            wrist: [0.0, 0.0, 0.03], palm: [0.0, 0.0, -1.0], fingers: [0.0, 1.0, 0.0], forearm: [0.0, -1.0, 0.5],
            approach: Some([0.0, 0.0, 1.0]),
            shape: [SLAP, SLAP],
            close: [&[], &[]],
            lands: &[],
            home: [[-0.013, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [-0.5, -1.0, 0.0]],
        }),
    },
    // Two fists meet knuckles to knuckles, thumbs on the same side.
    Style {
        name: "fist_bump",
        same: Some(Hold {
            wrist: [0.0, 0.21, 0.046], palm: [0.0, 0.0, -1.0], fingers: [0.0, -1.0, 0.0], forearm: [0.0, 1.0, 0.0],
            approach: Some([0.0, 1.0, 0.0]),
            shape: [FIST, FIST],
            close: [&[], &[]],
            lands: &[],
            home: [[-0.105, 0.0, 0.0], [0.0, 0.0, -1.0], [1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]],
        }),
        other: Some(Hold {
            wrist: [0.0, 0.21, 0.0], palm: [0.0, 0.0, 1.0], fingers: [0.0, -1.0, 0.0], forearm: [0.0, 1.0, 0.0],
            approach: Some([0.0, 1.0, 0.0]),
            shape: [FIST, FIST],
            close: [&[], &[]],
            lands: &[],
            home: [[-0.105, 0.0, 0.0], [0.0, 0.0, -1.0], [1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]],
        }),
    },
    // Pinky promise: two right hands in loose fists, thumbs up and resting on the index, the
    // follower turned half round about a line across the pinkies (crossing at 20 deg) and
    // 1.6 cm lower, each pinky bent into a hook round the other's middle segment.
    Style {
        name: "pinky",
        same: Some(Hold {
            wrist: [-0.0556, 0.2228, 0.04], palm: [0.0, 0.0, -1.0], fingers: [0.342, -0.9397, 0.0], forearm: [-0.342, 0.9397, 0.0],
            approach: None,
            shape: [PINKY_HOOK, PINKY_HOOK],
            close: [THUMB_PINKY, THUMB_PINKY],
            lands: &[(0, "pinky.distal", 1, "pinky.middle"), (1, "pinky.distal", 0, "pinky.middle"), (0, "thumb.pad", 0, "index.middle"), (1, "thumb.pad", 1, "index.middle")],
            home: [[-0.08, 0.0, 0.0], [0.0, 0.0, -1.0], [1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]],
        }),
        other: None,
    },
];

/// A right leader hanging at the side, palm toward the other person, thumb forward: walking
/// hand in hand.
const HANG: [[f64; 3]; 4] = [[-0.013, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [-0.6, 1.0, 0.0]];

pub const NAMES: [&str; 6] = ["handshake", "interlaced", "cupped", "high_five", "fist_bump", "pinky"];

/// A grip goal, read: the two hands, the style and the follower's frame in the leader's (and
/// its forearm direction).
struct Grip {
    ids: [HandId; 2],
    style: &'static Style,
    hold: &'static Hold,
    offset: DAffine3,
    forearm: DVec3,
    /// The digits that close, per hand: the style's, less those the goal's vary leaves alone.
    close: [Vec<Digit>; 2],
}

impl Grip {
    fn of(goal: &Goal) -> Result<Grip, String> {
        let [a, b] = goal.grip.as_ref().ok_or("not a grip")?;
        let (la, lb) = (HandId::parse(a)?, HandId::parse(b)?);
        if la == lb {
            return Err(format!("grip [{a}, {b}]: name two different hands"));
        }
        let name = goal.style.as_deref().ok_or_else(|| format!("grip [{a}, {b}] needs \"style\": one of {}", NAMES.join(", ")))?;
        let style = STYLES.iter().find(|s| s.name == name).ok_or_else(|| format!("grip style \"{name}\": use one of {}", NAMES.join(", ")))?;
        let hold = if la.side == lb.side { &style.same } else { &style.other };
        let Some(o) = hold else {
            let want = if style.same.is_some() { "two right hands or two left hands" } else { "a right and a left hand" };
            return Err(format!("grip {name} [{a}, {b}]: this style joins {want}"));
        };
        let mut offset = frame(o.wrist.into(), o.palm.into(), o.fingers.into());
        let mut forearm = DVec3::from(o.forearm).normalize();
        if la.side == Side::Left {
            // The same arrangement, mirrored: a left leader's thumb is on -x of its frame.
            let m = DAffine3::from_scale(DVec3::new(-1.0, 1.0, 1.0));
            offset = m * offset * m;
            forearm.x = -forearm.x;
        }
        let close = [0, 1].map(|who| {
            let name = [&la, &lb][who].name();
            o.close[who].iter().copied().filter(|d| joints(*d).iter().any(|(k, _, _)| lets_vary(goal, &name, k))).collect()
        });
        Ok(Grip { ids: [la, lb], style, hold: o, offset, forearm, close })
    }

    fn label(&self) -> String {
        format!("grip {} ~ {} ({})", self.ids[0], self.ids[1], self.style.name)
    }
}

/// The two hands a grip names (leader, follower), when it names two.
pub fn pair(goal: &Goal) -> Option<(HandId, HandId)> {
    let [a, b] = goal.grip.as_ref()?;
    Some((HandId::parse(a).ok()?, HandId::parse(b).ok()?))
}

/// The digits each hand of a grip closes (leader, follower), by name.
pub fn closing(goal: &Goal) -> Result<[Vec<&'static str>; 2], String> {
    let g = Grip::of(goal)?;
    Ok(g.close.map(|ds| ds.iter().map(|d| d.name()).collect()))
}

/// A hand frame from a placement: x = y cross z, y along the fingers, z out of the palm.
fn frame(wrist: DVec3, palm: DVec3, fingers: DVec3) -> DAffine3 {
    let y = fingers.normalize();
    let z = (palm - y * palm.dot(y)).normalize();
    DAffine3::from_mat3_translation(DMat3::from_cols(y.cross(z), y, z), wrist)
}

/// A hand's frame in the world: origin at the wrist joint, y along the fingers, z out of the
/// palm, x = y cross z (the thumb side of a right hand).
pub fn hand_frame(h: &HandGeom) -> DAffine3 {
    h.bones[1].frame
}

fn hand<'a>(s: &'a Solved, id: &HandId) -> Result<&'a HandGeom, String> {
    s.hands.iter().find(|h| h.id == *id).ok_or_else(|| format!("grip: there is no {id} hand in the file"))
}

fn gap(a: &Capsule, b: &Capsule) -> f64 {
    seg_dist(a.a, a.b, b.a, b.b) - a.r - b.r
}

/// A digit's segments, base to tip: what each of its closing joints moves.
fn segments(h: &HandGeom, d: Digit) -> Vec<&Capsule> {
    let order: &[Seg] = if d == Digit::Thumb { &[Seg::Metacarpal, Seg::Proximal, Seg::Distal] } else { &[Seg::Proximal, Seg::Middle, Seg::Distal] };
    order.iter().filter_map(|s| h.capsules.iter().find(|c| c.digit == Some(d) && c.seg == *s)).collect()
}

/// The closing joints of a digit and how fast each turns (base to tip, one per segment).
/// The thumb's base swings toward the fingers as it lifts, so it comes over what the hand holds.
fn joints(d: Digit) -> Vec<(String, f64, usize)> {
    let n = d.name();
    if d == Digit::Thumb {
        vec![("thumb.cmc_abd".into(), 0.6, 0), ("thumb.cmc_flex".into(), 0.5, 0), ("thumb.mcp_flex".into(), 0.6, 1), ("thumb.ip_flex".into(), 1.0, 2)]
    } else {
        vec![(format!("{n}.mcp_flex"), 1.0, 0), (format!("{n}.pip_flex"), 1.0, 1), (format!("{n}.dip_flex"), 0.75, 2)]
    }
}

/// The gap (m) from a capsule to the nearest part of `other`.
fn nearest(c: &Capsule, other: &HandGeom) -> f64 {
    other.capsules.iter().map(|o| gap(c, o)).fold(f64::INFINITY, f64::min)
}

/// The deepest overlap between two hands' parts (m, negative; 0 when none) and its pair.
fn deepest(a: &HandGeom, b: &HandGeom) -> (f64, String) {
    let mut out = (0.0, String::new());
    for ca in &a.capsules {
        for cb in &b.capsules {
            let g = gap(ca, cb);
            if g < out.0 {
                out = (g, format!("{} x {}", ca.label(), cb.label()));
            }
        }
    }
    out
}

/// The residuals (m) and status of a grip: every closing digit's tip on the other hand, the hands
/// touching, and nothing of one through the other.
pub fn evaluate(goal: &Goal, s: &Solved) -> Result<(Vec<f64>, GoalStatus), String> {
    let g = Grip::of(goal)?;
    let hands = [hand(s, &g.ids[0])?, hand(s, &g.ids[1])?];
    let mut res = Vec::new();
    let mut per: [Vec<String>; 2] = [Vec::new(), Vec::new()];
    let mut worst = 0.0_f64;
    let mut lands_met = true;
    for who in 0..2 {
        for d in &g.close[who] {
            if let Some(&(_, site, other, onto)) = g.hold.lands.iter().find(|l| l.0 == who && l.1.split('.').next() == Some(d.name())) {
                // A contact the style names: the digit lands there, like a touch goal.
                let touch = Goal { touch: Some([format!("{}.{site}", g.ids[who]), format!("{}.{onto}", g.ids[other])]), ..Goal::default() };
                let (r, st) = crate::goals::evaluate(&touch, s)?;
                res.extend(r);
                lands_met &= st.met;
                if st.value.abs() > worst.abs() {
                    worst = st.value;
                }
                let at = if other == who { onto.to_string() } else { format!("{}.{onto}", g.ids[other]) };
                per[who].push(format!("{} {:.1} on {at}", d.name(), st.value));
                continue;
            }
            let tip = *segments(hands[who], *d).last().unwrap();
            let mm = nearest(tip, hands[1 - who]) * 1000.0;
            res.push(mm / 1000.0);
            if mm.abs() > worst.abs() {
                worst = mm;
            }
            per[who].push(format!("{} {mm:.1}", d.name()));
        }
    }
    let touch = hands[0].capsules.iter().map(|c| nearest(c, hands[1])).fold(f64::INFINITY, f64::min) * 1000.0;
    res.push(touch.max(0.0) / 1000.0);
    if touch.max(0.0) > worst.abs() {
        worst = touch;
    }
    let (overlap, pair) = deepest(hands[0], hands[1]);
    let overlap_mm = -overlap * 1000.0;
    let met = lands_met && worst.abs() <= TOUCH_TOL_MM && overlap_mm <= PENETRATION_MM;
    let mut notes = Vec::new();
    for who in 0..2 {
        if !per[who].is_empty() {
            notes.push(format!("{} tips {}", g.ids[who], per[who].join(", ")));
        }
    }
    notes.push(format!("hands {:.1}", touch.max(0.0)));
    let mut note = format!("{} mm", notes.join("; "));
    if overlap_mm > 0.05 {
        note.push_str(&format!("; deepest overlap {overlap_mm:.1} mm ({pair})"));
    }
    Ok((res, GoalStatus { label: g.label(), value: (worst * 10.0).round() / 10.0, unit: "mm", met, note }))
}

/// What a grip varies when the solver runs on it after `prepare`: its closing digits and the
/// follower's placement.
pub fn vary(goal: &Goal) -> Result<Vec<String>, String> {
    let g = Grip::of(goal)?;
    let mut out: Vec<String> = Vec::new();
    for who in 0..2 {
        for d in &g.close[who] {
            out.push(format!("{}.{}.*", g.ids[who], d.name()));
        }
    }
    out.push(format!("{}.place", g.ids[1]));
    Ok(out)
}

/// Whether the grip may move hand `name`: its vary is left to the default (which moves the
/// follower) or names that hand's placement or everything on it.
fn moves_place(goal: &Goal, name: &str) -> bool {
    goal.vary.is_empty() || goal.vary.iter().any(|p| p.strip_prefix(name).and_then(|r| r.strip_prefix('.')).is_some_and(|r| r == "*" || r.starts_with("place")))
}

/// Whether the goal's vary lets the grip change `key` (a joint value such as "thumb.ip_flex") of
/// hand `name`: always when the vary is left to the default.
fn lets_vary(goal: &Goal, name: &str, key: &str) -> bool {
    goal.vary.is_empty()
        || goal.vary.iter().any(|p| {
            p.strip_prefix(name).and_then(|r| r.strip_prefix('.')).is_some_and(|r| match r {
                "*" => true,
                "wrist" => ["twist", "wrist_flex", "wrist_dev"].contains(&key),
                r if r.starts_with("*.") => key.split_once('.').is_some_and(|(_, j)| j == &r[2..]),
                r => r == key || key.split_once('.').is_some_and(|(d, _)| r == d || r.strip_suffix(".*") == Some(d)),
            })
        })
}

/// Whether a hand of the file has its own placement (or mirrors another's).
fn placed(raw: &Value, resolved: &Value, name: &str) -> bool {
    let r = &resolved["hands"][name];
    ["place", "elbow", "forearm_rotation_deg"].iter().any(|k| r.get(*k).is_some_and(|v| !v.is_null())) || raw["hands"][name].get("mirror").is_some()
}

fn arr(v: DVec3) -> Value {
    let r = |x: f64| (x * 1e5).round() / 1e5 + 0.0;
    json!([r(v.x), r(v.y), r(v.z)])
}

fn place_value(f: DAffine3, forearm: DVec3) -> Value {
    json!({ "wrist": arr(f.translation), "palm_facing": arr(f.matrix3.z_axis), "fingers_toward": arr(f.matrix3.y_axis), "forearm_toward": arr(forearm) })
}

fn set_place(work: &mut Value, name: &str, place: Value) {
    let h = &mut work["hands"][name];
    if !h.is_object() {
        *h = json!({});
    }
    let o = h.as_object_mut().unwrap();
    for k in ["elbow", "forearm_rotation_deg"] {
        o.remove(k);
    }
    o.insert("place".into(), place);
}

fn set_value(work: &mut Value, hand: &str, key: &str, x: f64) {
    let mut cur = &mut work["hands"][hand];
    let path: Vec<&str> = key.split('.').collect();
    for k in &path[..path.len() - 1] {
        if !cur.get(*k).is_some_and(Value::is_object) {
            cur[*k] = json!({});
        }
        cur = cur.get_mut(*k).unwrap();
    }
    cur[path[path.len() - 1]] = json!((x * 10.0).round() / 10.0 + 0.0);
}

fn get_value(resolved: &Value, hand: &str, key: &str) -> f64 {
    key.split('.').fold(&resolved["hands"][hand], |v, k| &v[k]).as_f64().unwrap_or(0.0)
}

/// Whether two hands overlap anywhere but the thumbs' outer segments (which the closing and the
/// solver move out of the way).
fn hands_overlap(s: &Solved, ids: &[HandId; 2]) -> Result<bool, String> {
    let (a, b) = (hand(s, &ids[0])?, hand(s, &ids[1])?);
    let solid = |c: &&Capsule| c.digit != Some(Digit::Thumb) || c.seg == Seg::Metacarpal;
    Ok(a.capsules.iter().filter(solid).any(|c| b.capsules.iter().filter(solid).any(|o| gap(c, o) < 0.0)))
}

/// The most proximal segment of digit `d` of hand `ids[who]` that overlaps the other hand or
/// another part of its own (0 base .. 2 tip).
fn first_hit(s: &Solved, ids: &[HandId; 2], who: usize, d: Digit) -> Result<Option<usize>, String> {
    let (own, other) = (hand(s, &ids[who])?, hand(s, &ids[1 - who])?);
    for (i, c) in segments(own, d).into_iter().enumerate() {
        let mine = own.capsules.iter().filter(|o| o.digit != Some(d) && crate::query::collides(c, o)).map(|o| gap(c, o)).fold(f64::INFINITY, f64::min);
        if nearest(c, other).min(mine) < 0.0 {
            return Ok(Some(i));
        }
    }
    Ok(None)
}

/// Degrees per closing step.
const STEP: f64 = 2.0;

/// Curls each closing digit of both hands, all its joints together, stopping each joint where
/// the segment it moves (or one beyond it) meets a surface, until every tip has landed or its
/// joints are at their limits.
fn close_digits(work: &mut Value, g: &Grip) -> Result<(), String> {
    let names = [g.ids[0].name(), g.ids[1].name()];
    // (hand, digit, joints frozen from the base)
    let mut runs: Vec<(usize, Digit, usize)> = (0..2).flat_map(|who| g.close[who].iter().map(move |d| (who, *d, 0))).collect();
    let limit = |k: &str| params::find(k).map(|p| (p.min, p.max)).unwrap();
    for _ in 0..(200.0 / STEP) as usize {
        let mut moving = false;
        for run in runs.iter_mut() {
            let (who, d, frozen) = *run;
            if frozen >= 3 {
                continue;
            }
            let (resolved, _) = crate::pose::resolve(work.clone())?;
            let js = joints(d);
            let cur: Vec<f64> = js.iter().map(|(k, _, _)| get_value(&resolved, &names[who], k)).collect();
            let next: Vec<f64> = js.iter().enumerate().map(|(i, (k, rate, seg))| if *seg < frozen { cur[i] } else { (cur[i] + rate * STEP).clamp(limit(k).0, limit(k).1) }).collect();
            if next.iter().zip(&cur).all(|(a, b)| (a - b).abs() < 1e-9) {
                run.2 = 3;
                continue;
            }
            moving = true;
            let put = |work: &mut Value, f: f64| {
                for (i, (k, _, _)) in js.iter().enumerate() {
                    set_value(work, &names[who], k, cur[i] + (next[i] - cur[i]) * f);
                }
            };
            put(work, 1.0);
            let Some(seg) = first_hit(&crate::solve_value(work)?, &g.ids, who, d)? else { continue };
            // Back off to where it touches.
            let (mut lo, mut hi) = (0.0, 1.0);
            for _ in 0..6 {
                let mid = (lo + hi) / 2.0;
                put(work, mid);
                if first_hit(&crate::solve_value(work)?, &g.ids, who, d)?.is_some() { hi = mid } else { lo = mid }
            }
            put(work, lo);
            run.2 = run.2.max(seg + 1);
        }
        if !moving {
            break;
        }
    }
    Ok(())
}

/// Before solving: every grip that does not hold places its follower by the style (or its
/// leader, when only the follower is placed; or both, when neither is), sets both hands to the
/// style's open shape, slides the follower in until the hands touch and closes the gripping
/// digits around the other hand. Returns the file to solve from.
pub fn prepare(raw: &Value) -> Result<Value, String> {
    let mut work = raw.clone();
    let first = crate::solve_value(raw)?;
    let goals: Vec<Goal> = first.pose.goals.iter().filter(|g| g.grip.is_some()).cloned().collect();
    for goal in goals {
        if evaluate(&goal, &crate::solve_value(&work)?)?.1.met {
            continue;
        }
        let g = Grip::of(&goal)?;
        let names = [g.ids[0].name(), g.ids[1].name()];
        for (who, name) in names.iter().enumerate() {
            for (key, x) in g.hold.shape[who].iter().filter(|(k, _)| lets_vary(&goal, name, k)) {
                set_value(&mut work, name, key, *x);
            }
        }
        let (resolved, _) = crate::pose::resolve(work.clone())?;
        let (pa, pb) = (placed(&work, &resolved, &names[0]), placed(&work, &resolved, &names[1]));
        if pa && pb && !names.iter().any(|n| moves_place(&goal, n)) {
            // Both hands placed and the goal's own vary keeps them there: only the digits close.
            close_digits(&mut work, &g)?;
            continue;
        }
        if !pa && !pb {
            let [w, p, f, a] = g.hold.home;
            let x = if g.ids[0].side == Side::Left { -1.0 } else { 1.0 };
            let m = |v: [f64; 3]| json!([v[0] * x, v[1], v[2]]);
            set_place(&mut work, &names[0], json!({ "wrist": m(w), "palm_facing": m(p), "fingers_toward": m(f), "forearm_toward": m(a) }));
        }
        let s = crate::solve_value(&work)?;
        // The style's offset is for two hands of the default size; for a smaller or larger
        // person's hand the follower sits nearer or farther, by the two hands' mean size.
        let mut offset = g.offset;
        offset.translation *= (hand(&s, &g.ids[0])?.scale + hand(&s, &g.ids[1])?.scale) / 2.0;
        // Where the follower goes, moved `d` back along the approach.
        let put: Box<dyn Fn(&mut Value, f64)> = if pa || !pb {
            let l = hand_frame(hand(&s, &g.ids[0])?);
            let (off, fore) = (offset, l.transform_vector3(g.forearm));
            let back = l.transform_vector3(DVec3::from(g.hold.approach.unwrap_or([0.0; 3])));
            let name = names[1].clone();
            Box::new(move |w: &mut Value, d: f64| set_place(w, &name, place_value(DAffine3::from_translation(back * d) * l * off, fore)))
        } else {
            // Only the follower is placed: the leader goes where the follower's frame needs it,
            // its forearm straight back from the hand.
            let f = hand_frame(hand(&s, &g.ids[1])?);
            let l = f * offset.inverse();
            let back = l.transform_vector3(DVec3::from(g.hold.approach.unwrap_or([0.0; 3])));
            let name = names[0].clone();
            Box::new(move |w: &mut Value, d: f64| {
                let moved = DAffine3::from_translation(-back * d) * l;
                set_place(w, &name, place_value(moved, moved.transform_vector3(DVec3::NEG_Y)))
            })
        };
        if g.hold.approach.is_some() {
            // Slide in from 6 cm back until the hands touch.
            let (mut near, mut far) = (-0.05, 0.06);
            put(&mut work, far);
            if hands_overlap(&crate::solve_value(&work)?, &g.ids)? {
                far = 0.15;
            }
            for _ in 0..24 {
                let mid = (near + far) / 2.0;
                put(&mut work, mid);
                if hands_overlap(&crate::solve_value(&work)?, &g.ids)? { near = mid } else { far = mid }
            }
            put(&mut work, far);
            if std::env::var_os("HANDKIT_DEBUG").is_some() {
                let s = crate::solve_value(&work)?;
                let (a, b) = (hand(&s, &g.ids[0])?, hand(&s, &g.ids[1])?);
                let mut pairs: Vec<(f64, String)> = a.capsules.iter().flat_map(|x| b.capsules.iter().map(move |y| (gap(x, y), format!("{} ~ {}", x.label(), y.label())))).collect();
                pairs.sort_by(|x, y| x.0.total_cmp(&y.0));
                eprintln!("grip {}: slid in to {far:.4} m; nearest {:?}", g.style.name, &pairs[..3]);
            }
        } else {
            put(&mut work, 0.0);
        }
        close_digits(&mut work, &g)?;
        if std::env::var_os("HANDKIT_DEBUG").is_some() {
            eprintln!("closed: {}", evaluate(&goal, &crate::solve_value(&work)?)?.1.line());
        }
    }
    Ok(work)
}
