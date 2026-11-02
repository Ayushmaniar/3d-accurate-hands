//! The hand rig and its forward kinematics. Everything is computed for a RIGHT hand in the
//! forearm frame (elbow at the origin, forearm along +Y, palm facing +Z, thumb on +X), then
//! mirrored in X for a left hand and placed in the world.

use glam::{DAffine3, DMat3, DQuat, DVec3, dvec3};

use crate::params;
use std::sync::Arc;

use crate::pose::{HandId, HandPose, Side};

pub const FOREARM: f64 = 0.26;
/// The forearm capsule is as thick as a wrist (front to back), not as wide, and stops short of
/// the wrist joint so its rounded end does not bulge past the heel of the palm.
const FOREARM_R: f64 = 0.022;
const FOREARM_END: f64 = 0.012;
const METACARPAL_R: f64 = 0.0125;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Digit {
    Thumb,
    Index,
    Middle,
    Ring,
    Pinky,
}

impl Digit {
    pub const ALL: [Digit; 5] = [Digit::Thumb, Digit::Index, Digit::Middle, Digit::Ring, Digit::Pinky];

    pub fn name(self) -> &'static str {
        match self {
            Digit::Thumb => "thumb",
            Digit::Index => "index",
            Digit::Middle => "middle",
            Digit::Ring => "ring",
            Digit::Pinky => "pinky",
        }
    }
}

struct FingerDims {
    digit: Digit,
    /// Knuckle (MCP) position in the hand frame.
    mcp: [f64; 2],
    /// Base of the metacarpal in the hand frame.
    base: [f64; 2],
    /// Rest spread in degrees, positive toward the thumb.
    fan: f64,
    len: [f64; 3],
    r: f64,
}

const FINGERS: [FingerDims; 4] = [
    FingerDims { digit: Digit::Index, mcp: [0.030, 0.093], base: [0.012, 0.010], fan: 4.0, len: [0.040, 0.025, 0.020], r: 0.0095 },
    FingerDims { digit: Digit::Middle, mcp: [0.010, 0.096], base: [0.004, 0.012], fan: 0.0, len: [0.045, 0.028, 0.021], r: 0.0098 },
    FingerDims { digit: Digit::Ring, mcp: [-0.010, 0.091], base: [-0.006, 0.012], fan: -4.0, len: [0.042, 0.027, 0.021], r: 0.0092 },
    FingerDims { digit: Digit::Pinky, mcp: [-0.029, 0.083], base: [-0.015, 0.010], fan: -9.0, len: [0.033, 0.019, 0.018], r: 0.0082 },
];

pub const THUMB_CMC: [f64; 3] = [0.020, 0.018, 0.008];
pub const THUMB_LEN: [f64; 3] = [0.046, 0.034, 0.027];
pub const THUMB_R: [f64; 3] = [0.0135, 0.0105, 0.0095];
/// Rest orientation of the thumb's base, applied in this order in the hand frame:
/// lean toward the thumb side, lift out of the palm, turn the pad toward the index.
pub const THUMB_REST_DEG: [f64; 3] = [-35.0, 10.0, -55.0];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seg {
    Arm,
    Metacarpal,
    Proximal,
    Middle,
    Distal,
}

impl Seg {
    pub fn name(self) -> &'static str {
        match self {
            Seg::Arm => "arm",
            Seg::Metacarpal => "metacarpal",
            Seg::Proximal => "proximal",
            Seg::Middle => "middle",
            Seg::Distal => "distal",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Capsule {
    pub a: DVec3,
    pub b: DVec3,
    pub r: f64,
    pub side: Side,
    /// The hand's name ("right", "ana.left"), which starts its label.
    pub hand: Arc<str>,
    /// "forearm", "palm" (finger metacarpals) or the digit name.
    pub part: &'static str,
    /// The digit a segment belongs to, metacarpals included.
    pub digit: Option<Digit>,
    pub seg: Seg,
}

impl Capsule {
    pub fn label(&self) -> String {
        match self.digit {
            Some(d) => format!("{}.{}.{}", self.hand, d.name(), self.seg.name()),
            None => format!("{}.forearm", self.hand),
        }
    }

    /// A finger or thumb segment that moves (not the forearm or a metacarpal).
    pub fn is_phalanx(&self) -> bool {
        self.digit.is_some() && self.seg != Seg::Metacarpal
    }
}

#[derive(Clone, Debug)]
pub struct Site {
    pub name: String,
    pub pos: DVec3,
    pub normal: DVec3,
}

#[derive(Clone, Debug)]
pub struct Bone {
    pub name: String,
    pub origin: DVec3,
    pub along: DVec3,
    pub palmar: DVec3,
    pub len: f64,
    /// The bone's frame in the world, always a proper rotation (a left hand's frames are the
    /// right hand's reflected through x, so a mesh bound to the mirrored rest pose skins
    /// correctly): y along the bone, z toward the palm side.
    pub frame: DAffine3,
    /// The bone it hangs from (an index into `bones`); the forearm has none.
    pub parent: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct Nail {
    pub pos: DVec3,
    pub up: DVec3,
    pub out: DVec3,
    pub w: f64,
}

#[derive(Clone, Debug)]
pub struct HandGeom {
    pub side: Side,
    /// Whose hand: the default person's "right" / "left", or e.g. "ana.right".
    pub id: HandId,
    /// `id` written out, as sites and labels start with it.
    pub name: Arc<str>,
    pub capsules: Vec<Capsule>,
    pub sites: Vec<Site>,
    pub bones: Vec<Bone>,
    pub nails: Vec<Nail>,
    pub violations: Vec<String>,
    /// Values filled in by couplings, e.g. "index.dip_flex=40 (2/3 of pip)".
    pub coupled: Vec<String>,
    /// Placement: world = world * (mirror for a left hand) * local.
    pub world: DAffine3,
    /// The wrist_flex and wrist_dev the hand was built with (`place.forearm_toward` sets them).
    pub wrist_angles: [f64; 2],
    /// Whether `place.forearm_toward` set the wrist angles.
    pub forearm_aimed: bool,
}

/// A skeleton joint's transform relative to its parent (the forearm's is in the world).
#[derive(Clone, Copy, Debug)]
pub struct JointLocal {
    pub translation: DVec3,
    pub rotation: DQuat,
}

/// Corrective blend shapes of the hand mesh, in mesh target order: (bone at the joint, the bend
/// in degrees where the shape is fully on). The shape bulges the back of the joint and tucks in
/// the palm side, which plain skinning flattens.
pub const CORRECTIVES: [(&str, f64); 14] = [
    ("index_mcp", 90.0), ("index_pip", 100.0), ("index_dip", 70.0),
    ("middle_mcp", 90.0), ("middle_pip", 100.0), ("middle_dip", 70.0),
    ("ring_mcp", 90.0), ("ring_pip", 100.0), ("ring_dip", 70.0),
    ("pinky_mcp", 90.0), ("pinky_pip", 100.0), ("pinky_dip", 70.0),
    ("thumb_mcp", 60.0), ("thumb_ip", 80.0),
];

impl HandGeom {
    /// How far each corrective shape is on (0..1, CORRECTIVES order): the joint's bend toward
    /// the palm over the bend where the shape is full.
    pub fn corrective_weights(&self) -> Vec<f64> {
        CORRECTIVES
            .iter()
            .map(|(name, full)| {
                let Some(b) = self.bones.iter().find(|b| b.name == *name) else { return 0.0 };
                let parent = &self.bones[b.parent.unwrap()];
                let y = parent.frame.inverse().transform_vector3(b.frame.transform_vector3(DVec3::Y));
                let bend = y.z.atan2(y.y).to_degrees();
                (bend / full).clamp(0.0, 1.0)
            })
            .collect()
    }

    /// The glTF joint name of a bone: "index_pip_R".
    pub fn joint_name(&self, bone: usize) -> String {
        format!("{}_{}", self.bones[bone].name, if self.side == Side::Left { "L" } else { "R" })
    }

    /// Every bone's transform relative to its parent, in `bones` order: what a skinned mesh
    /// bound to this skeleton needs per frame.
    pub fn locals(&self) -> Vec<JointLocal> {
        self.bones
            .iter()
            .map(|b| {
                let local = match b.parent {
                    Some(p) => self.bones[p].frame.inverse() * b.frame,
                    None => b.frame,
                };
                JointLocal { translation: local.translation, rotation: DQuat::from_mat3(&local.matrix3).normalize() }
            })
            .collect()
    }
}

fn rx(d: f64) -> DAffine3 {
    DAffine3::from_rotation_x(d.to_radians())
}
fn ry(d: f64) -> DAffine3 {
    DAffine3::from_rotation_y(d.to_radians())
}
fn rz(d: f64) -> DAffine3 {
    DAffine3::from_rotation_z(d.to_radians())
}
fn tr(x: f64, y: f64, z: f64) -> DAffine3 {
    DAffine3::from_translation(dvec3(x, y, z))
}
fn mirror_x() -> DAffine3 {
    DAffine3::from_scale(dvec3(-1.0, 1.0, 1.0))
}

fn v(o: Option<f64>) -> f64 {
    o.unwrap_or(0.0)
}

/// Builds one of the default person's hands. `mirror_of` is the other hand when this one takes
/// its mirrored placement.
pub fn build(side: Side, hp: &HandPose, version: Option<u32>, mirror_of: Option<&HandGeom>) -> Result<HandGeom, String> {
    build_hand(&HandId::of(side), hp, version, mirror_of)
}

/// Builds any person's hand.
pub fn build_hand(id: &HandId, hp: &HandPose, version: Option<u32>, mirror_of: Option<&HandGeom>) -> Result<HandGeom, String> {
    let side = id.side;
    let name: Arc<str> = id.name().into();
    let prefix = Arc::clone(&name);
    let mut violations = Vec::new();
    let mut check = |key: &str, value: f64| {
        let p = params::find(key).expect("every checked key is in PARAMS");
        if value < p.min - 0.01 || value > p.max + 0.01 {
            violations.push(format!("{}.{}={} outside [{}, {}]", name, key, value, p.min, p.max));
        }
    };
    let twist = v(hp.twist);
    let mut wrist_flex = v(hp.wrist_flex);
    let mut wrist_dev = v(hp.wrist_dev);
    let own_aim = hp.place.as_ref().and_then(|p| p.forearm_toward).is_some();
    let mut forearm_aimed = false;
    if let Some((f, d)) = forearm_angles(side, &name, hp)? {
        (wrist_flex, wrist_dev, forearm_aimed) = (f, d, true);
    } else if let Some(other) = mirror_of.filter(|o| o.forearm_aimed && !own_aim) {
        // A mirror of a hand whose forearm was aimed takes the angles that aim gave.
        [wrist_flex, wrist_dev] = other.wrist_angles;
        forearm_aimed = true;
    }
    let cup = v(hp.palm_cup);
    check("twist", twist);
    check("wrist_flex", wrist_flex);
    check("wrist_dev", wrist_dev);
    check("palm_cup", cup);

    let wrist = tr(0.0, FOREARM, 0.0) * ry(twist);
    let hand = wrist * rz(-wrist_dev) * rx(wrist_flex);
    let left = side == Side::Left;
    let world = match mirror_of {
        Some(other) => mirror_x() * other.world * mirror_x(),
        None => placement(side, &name, hp, hand)?,
    };
    let to_world = |m: DAffine3| world * if left { mirror_x() * m } else { m };
    let at = |m: DAffine3, p: DVec3| to_world(m).transform_point3(p);
    let dir = |m: DAffine3, d: DVec3| to_world(m).transform_vector3(d).normalize();

    let mut capsules = Vec::new();
    let mut sites = Vec::new();
    let mut bones = Vec::new();
    let mut nails = Vec::new();
    let mut coupled = Vec::new();
    let o = DVec3::ZERO;
    let mut capsule = |a: DVec3, b: DVec3, r: f64, part: &'static str, digit: Option<Digit>, seg: Seg| {
        capsules.push(Capsule { a, b, r, side, hand: Arc::clone(&name), part, digit, seg });
    };
    let mut bone = |name: String, m: DAffine3, len: f64, parent: Option<usize>| {
        let frame = to_world(m) * if left { mirror_x() } else { DAffine3::IDENTITY };
        bones.push(Bone { name, origin: at(m, o), along: dir(m, DVec3::Y), palmar: dir(m, DVec3::Z), len, frame, parent });
        bones.len() - 1
    };
    let mut site = |name: String, m: DAffine3, p: DVec3, n: DVec3| {
        sites.push(Site { name: format!("{prefix}.{name}"), pos: at(m, p), normal: dir(m, n) });
    };

    capsule(at(DAffine3::IDENTITY, o), at(wrist, dvec3(0.0, -FOREARM_END, 0.0)), FOREARM_R, "forearm", None, Seg::Arm);
    let forearm_bone = bone("forearm".into(), DAffine3::IDENTITY, FOREARM, None);
    let hand_bone = bone("hand".into(), hand, 0.095, Some(forearm_bone));
    site("palm_center".into(), hand, dvec3(0.0, 0.055, 0.013), DVec3::Z);
    site("palm_heel".into(), hand, dvec3(0.0, 0.02, 0.013), DVec3::Z);
    site("wrist_back".into(), hand, dvec3(0.0, 0.0, -0.016), DVec3::NEG_Z);

    let digit_pose = |d: Digit| match d {
        Digit::Index => hp.index.clone().unwrap_or_default(),
        Digit::Middle => hp.middle.clone().unwrap_or_default(),
        Digit::Ring => hp.ring.clone().unwrap_or_default(),
        _ => hp.pinky.clone().unwrap_or_default(),
    };
    for f in &FINGERS {
        let name = f.digit.name();
        let q = digit_pose(f.digit);
        let pip = v(q.pip_flex);
        let dip = match q.dip_flex {
            Some(d) => d,
            None if version.unwrap_or(0) >= 1 => {
                let dip_max = params::find("index.dip_flex").unwrap().max;
                let d = ((pip * 2.0 / 3.0 * 10.0).round() / 10.0).min(dip_max);
                if d != 0.0 {
                    coupled.push(format!("{}.{}.dip_flex={} (2/3 of pip_flex)", prefix, name, d));
                }
                d
            }
            None => 0.0,
        };
        let flex = [v(q.mcp_flex), pip, dip];
        let abd = v(q.mcp_abd);
        let roll = v(q.mcp_roll);
        check(&format!("{name}.mcp_flex"), flex[0]);
        check(&format!("{name}.pip_flex"), flex[1]);
        check(&format!("{name}.dip_flex"), flex[2]);
        check(&format!("{name}.mcp_abd"), abd);
        check(&format!("{name}.mcp_roll"), roll);

        let cup_k = match f.digit {
            Digit::Pinky => 1.0,
            Digit::Ring => 0.5,
            _ => 0.0,
        };
        let base = hand * tr(f.base[0], f.base[1], 0.0);
        let mut m = hand * tr(f.mcp[0], f.mcp[1], 0.03 * (cup * cup_k).to_radians().sin()) * ry(cup * cup_k);
        capsule(at(base, o), at(m, o), METACARPAL_R, "palm", Some(f.digit), Seg::Metacarpal);
        // Positive abduction spreads away from the middle finger (middle: toward the index).
        let abd_sign = if matches!(f.digit, Digit::Ring | Digit::Pinky) { 1.0 } else { -1.0 };
        // Positive roll turns the pad toward the thumb (+X in the right hand's frame).
        m = m * rz(-f.fan + abd_sign * abd) * ry(roll);
        let segs = [Seg::Proximal, Seg::Middle, Seg::Distal];
        let joints = ["mcp", "pip", "dip"];
        let mut parent = hand_bone;
        for i in 0..3 {
            m = m * rx(flex[i]);
            let r = f.r * (1.0 - 0.1 * i as f64);
            let next = m * tr(0.0, f.len[i], 0.0);
            capsule(at(m, o), at(next, o), r, name, Some(f.digit), segs[i]);
            parent = bone(format!("{name}_{}", joints[i]), m, f.len[i], Some(parent));
            match segs[i] {
                Seg::Proximal => site(format!("{name}.knuckle"), m, dvec3(0.0, 0.0, -r), DVec3::NEG_Z),
                Seg::Middle => {
                    site(format!("{name}.dip_radial"), next, dvec3(r, 0.0, 0.0), DVec3::X);
                    site(format!("{name}.dip_ulnar"), next, dvec3(-r, 0.0, 0.0), DVec3::NEG_X);
                }
                _ => {
                    let l = f.len[2];
                    site(format!("{name}.tip"), m, dvec3(0.0, l + r, 0.0), DVec3::Y);
                    site(format!("{name}.pad"), m, dvec3(0.0, 0.55 * l, r), DVec3::Z);
                    site(format!("{name}.nail"), m, dvec3(0.0, 0.6 * l, -r), DVec3::NEG_Z);
                    site(format!("{name}.side_radial"), m, dvec3(r, 0.5 * l, 0.0), DVec3::X);
                    site(format!("{name}.side_ulnar"), m, dvec3(-r, 0.5 * l, 0.0), DVec3::NEG_X);
                    nails.push(Nail { pos: at(m, dvec3(0.0, 0.6 * l, -0.75 * r)), up: dir(m, DVec3::Y), out: dir(m, DVec3::NEG_Z), w: r * 1.5 });
                }
            }
            m = next;
        }
    }

    let t = hp.thumb.clone().unwrap_or_default();
    let (cmc_flex, cmc_abd, cmc_rot) = (v(t.cmc_flex), v(t.cmc_abd), v(t.cmc_rot));
    let tflex = [0.0, v(t.mcp_flex), v(t.ip_flex)];
    check("thumb.cmc_flex", cmc_flex);
    check("thumb.cmc_abd", cmc_abd);
    check("thumb.cmc_rot", cmc_rot);
    check("thumb.mcp_flex", tflex[1]);
    check("thumb.ip_flex", tflex[2]);
    let [lean, lift, turn] = THUMB_REST_DEG;
    let mut m = hand
        * tr(THUMB_CMC[0], THUMB_CMC[1], THUMB_CMC[2])
        * rz(lean + cmc_flex)
        * rx(lift + cmc_abd)
        * ry(turn - cmc_rot);
    let segs = [Seg::Metacarpal, Seg::Proximal, Seg::Distal];
    let joints = ["cmc", "mcp", "ip"];
    let mut parent = hand_bone;
    for i in 0..3 {
        m = m * rx(tflex[i]);
        let r = THUMB_R[i];
        let next = m * tr(0.0, THUMB_LEN[i], 0.0);
        capsule(at(m, o), at(next, o), r, "thumb", Some(Digit::Thumb), segs[i]);
        parent = bone(format!("thumb_{}", joints[i]), m, THUMB_LEN[i], Some(parent));
        match segs[i] {
            Seg::Proximal => site("thumb.knuckle".into(), m, dvec3(0.0, 0.0, -r), DVec3::NEG_Z),
            Seg::Distal => {
                let l = THUMB_LEN[2];
                site("thumb.tip".into(), m, dvec3(0.0, l + r, 0.0), DVec3::Y);
                site("thumb.pad".into(), m, dvec3(0.0, 0.55 * l, r), DVec3::Z);
                site("thumb.nail".into(), m, dvec3(0.0, 0.6 * l, -r), DVec3::NEG_Z);
                // The pad faces the index, so the thumb's edges face the palm side (+X local,
                // +Z at rest) and the back of the hand.
                site("thumb.side_palmar".into(), m, dvec3(r, 0.5 * l, 0.0), DVec3::X);
                site("thumb.side_dorsal".into(), m, dvec3(-r, 0.5 * l, 0.0), DVec3::NEG_X);
                nails.push(Nail { pos: at(m, dvec3(0.0, 0.6 * l, -0.75 * r)), up: dir(m, DVec3::Y), out: dir(m, DVec3::NEG_Z), w: r * 1.5 });
            }
            _ => {}
        }
        m = next;
    }

    Ok(HandGeom { side, id: id.clone(), name, capsules, sites, bones, nails, violations, coupled, world, wrist_angles: [wrist_flex, wrist_dev], forearm_aimed })
}

/// The wrist_flex and wrist_dev that make the forearm point along `place.forearm_toward`
/// (from the wrist toward the elbow), for a hand placed by `place.wrist`. In the hand's own
/// frame the forearm points along (sin dev, -cos dev cos flex, cos dev sin flex).
fn forearm_angles(side: Side, s: &str, hp: &HandPose) -> Result<Option<(f64, f64)>, String> {
    let Some(p) = &hp.place else { return Ok(None) };
    let Some(toward) = p.forearm_toward else { return Ok(None) };
    let (Some(_), Some(palm), Some(fingers)) = (p.wrist, p.palm_facing, p.fingers_toward) else {
        return Err(format!("hands.{s}.place.forearm_toward needs wrist, palm_facing and fingers_toward"));
    };
    let y = DVec3::from(fingers).normalize_or_zero();
    let palm = DVec3::from(palm);
    let z = (palm - y * palm.dot(y)).normalize_or_zero();
    let d = DVec3::from(toward).normalize_or_zero();
    if y == DVec3::ZERO || z == DVec3::ZERO || d == DVec3::ZERO {
        return Err(format!("hands.{s}.place: forearm_toward must not be zero"));
    }
    let x = y.cross(z);
    let mut v = DVec3::new(d.dot(x), d.dot(y), d.dot(z));
    if side == Side::Left {
        v.x = -v.x;
    }
    let dev = v.x.clamp(-1.0, 1.0).asin().to_degrees();
    let flex = v.z.atan2(-v.y).to_degrees();
    Ok(Some((flex, dev)))
}

/// The world transform of the forearm frame from the hand's placement.
fn placement(side: Side, s: &str, hp: &HandPose, hand: DAffine3) -> Result<DAffine3, String> {
    let p = hp.place.clone().unwrap_or_default();
    let elbow = p.elbow.or(hp.elbow);
    let euler = p.forearm_rotation_deg.or(hp.forearm_rotation_deg);
    if let Some(wrist) = p.wrist {
        if elbow.is_some() || euler.is_some() {
            return Err(format!("hands.{s}.place: give either wrist + palm_facing + fingers_toward, or elbow + forearm_rotation_deg, not both"));
        }
        let (Some(palm), Some(fingers)) = (p.palm_facing, p.fingers_toward) else {
            return Err(format!("hands.{s}.place.wrist needs palm_facing and fingers_toward"));
        };
        let y = DVec3::from(fingers).normalize_or_zero();
        let palm = DVec3::from(palm);
        let z = (palm - y * palm.dot(y)).normalize_or_zero();
        if y == DVec3::ZERO || z == DVec3::ZERO {
            return Err(format!("hands.{s}.place: palm_facing must not be zero or parallel to fingers_toward"));
        }
        let desired = DAffine3::from_mat3_translation(DMat3::from_cols(y.cross(z), y, z), DVec3::from(wrist));
        let local = if side == Side::Left { mirror_x() * hand * mirror_x() } else { hand };
        return Ok(desired * local.inverse());
    }
    if p.palm_facing.is_some() || p.fingers_toward.is_some() {
        return Err(format!("hands.{s}.place: palm_facing and fingers_toward need wrist"));
    }
    let default_x = if side == Side::Left { -0.1 } else { 0.1 };
    let e = euler.unwrap_or([0.0; 3]);
    let rot = DMat3::from_rotation_x(e[0].to_radians())
        * DMat3::from_rotation_y(e[1].to_radians())
        * DMat3::from_rotation_z(e[2].to_radians());
    Ok(DAffine3::from_mat3_translation(rot, DVec3::from(elbow.unwrap_or([default_x, -0.3, 0.0]))))
}
