//! Goal residuals: how far each goal is from true, as numbers the solver drives to zero and
//! `query` reports.

use glam::DVec3;

use crate::Solved;
use crate::pose::Goal;
use crate::query::seg_closest;
use crate::rig::{Digit, Seg};

/// A goal counts as met within these.
pub const TOUCH_TOL_MM: f64 = 1.0;
pub const AIM_TOL_DEG: f64 = 1.0;
/// Meters of residual per unit of sin(aim angle): 1 deg weighs like 3.5 mm.
const AIM_WEIGHT: f64 = 0.2;
/// A touch named by a site is met when the surfaces meet within this of the site.
pub const SITE_TOL_MM: f64 = 6.0;
/// Meters of residual per meter the contact point strays from a named site beyond SITE_FREE_MM.
/// The rig's fingers do not roll, so two pads cannot always meet dead center; a few millimeters
/// toward the side of a pad (as in a real pinch) cost nothing, so the solver keeps the shape the
/// file started from instead of contorting the hand to center the contact.
const SITE_WEIGHT: f64 = 1.0;
const SITE_FREE_MM: f64 = 3.0;
/// A clear goal holds while no two parts overlap by more than this; the solver then pushes them
/// to within the overlap slack (0.5 mm), so turning it on barely moves anything.
const CLEAR_TOL_MM: f64 = 0.9;

pub struct GoalStatus {
    pub label: String,
    /// Touch: the gap in mm (negative = overlap). Aim: the angle in degrees.
    pub value: f64,
    pub unit: &'static str,
    pub met: bool,
    /// Where a touch landed, when it strays from a named site, e.g. "4.1 mm from right.index.pad".
    pub note: String,
}

impl GoalStatus {
    pub fn line(&self) -> String {
        let note = if self.note.is_empty() { String::new() } else { format!(" ({})", self.note) };
        format!("{} {}: {} {}{note}", if self.met { "ok  " } else { "MISS" }, self.label, self.value, self.unit)
    }
}

/// What a touch name refers to: a capsule (a site's own segment, or a named segment), and for a
/// site, the point on it the contact should land at. Palm sites are bare points.
pub struct Target {
    pub a: DVec3,
    pub b: DVec3,
    pub r: f64,
    pub site: Option<(String, DVec3)>,
}

/// The segment a digit site sits on: "right.index.pad" -> "right.index.distal".
fn site_segment(name: &str) -> Option<String> {
    let (rest, kind) = name.rsplit_once('.')?;
    let seg = match kind {
        "tip" | "pad" | "nail" | "side_radial" | "side_ulnar" | "side_palmar" | "side_dorsal" => "distal",
        "knuckle" => "proximal",
        "dip_radial" | "dip_ulnar" => "middle",
        _ => return None,
    };
    Some(format!("{rest}.{seg}"))
}

impl Solved {
    pub fn target(&self, name: &str) -> Result<Target, String> {
        let capsule = |label: &str| self.hands.iter().flat_map(|h| &h.capsules).find(|c| c.label() == label);
        if let Some(s) = self.hands.iter().flat_map(|h| &h.sites).find(|s| s.name == name) {
            return Ok(match site_segment(name).and_then(|l| capsule(&l)) {
                Some(c) => Target { a: c.a, b: c.b, r: c.r, site: Some((s.name.clone(), s.pos)) },
                None => Target { a: s.pos, b: s.pos, r: 0.0, site: None },
            });
        }
        if let Some(c) = capsule(name) {
            return Ok(Target { a: c.a, b: c.b, r: c.r, site: None });
        }
        Err(format!("unknown site or segment \"{name}\". Sites are <hand>.<digit>.<tip|pad|nail|knuckle>, <hand>.<index|middle|ring|pinky>.<side_radial|side_ulnar|dip_radial|dip_ulnar>, <hand>.thumb.<side_palmar|side_dorsal>, and <hand>.<palm_center|palm_heel|wrist_back>. Segments are <hand>.<digit>.<metacarpal|proximal|middle|distal> (the thumb has metacarpal, proximal, distal) and <hand>.forearm; <hand> is right, left, or <person>.right / <person>.left. The hand must be in the file"))
    }

    /// A finger's pointing line: from its knuckle through its tip.
    pub fn finger_ray(&self, name: &str) -> Result<(DVec3, DVec3), String> {
        let (id, digit) = crate::pose::split_hand(name).filter(|(_, d)| !d.is_empty()).ok_or(format!("aim \"{name}\": write <hand>.<digit>, e.g. right.index"))?;
        let d = Digit::ALL.into_iter().find(|d| d.name() == digit).ok_or(format!("aim \"{name}\": unknown digit"))?;
        let side = id.name();
        let h = self.hands.iter().find(|h| h.id == id).ok_or(format!("aim \"{name}\": no {side} hand in the file"))?;
        let base = h.capsules.iter().find(|c| c.digit == Some(d) && c.seg == Seg::Proximal).unwrap().a;
        let tip = h.sites.iter().find(|s| s.name == format!("{side}.{digit}.tip")).unwrap().pos;
        Ok((base, (tip - base).normalize()))
    }
}

/// The residual components of one goal (meters) and its status. A touch asks for the surfaces
/// to meet (or to stay `gap_mm` apart) and, for each named site, for the contact to land there.
pub fn evaluate(goal: &Goal, s: &Solved) -> Result<(Vec<f64>, GoalStatus), String> {
    if goal.grip.is_some() {
        if goal.touch.is_some() || goal.aim.is_some() {
            return Err("a grip is its own goal: write { \"grip\": [a, b], \"style\": .. } without touch or aim".into());
        }
        return crate::grip::evaluate(goal, s);
    }
    if goal.style.is_some() {
        return Err("\"style\" goes with \"grip\": [a, b]".into());
    }
    match (&goal.touch, &goal.aim) {
        (Some([a, b]), None) => {
            let want = goal.gap_mm.unwrap_or(0.0) / 1000.0;
            let label = format!("touch {a} ~ {b}");
            let (mut ta, mut tb) = (s.target(a)?, s.target(b)?);
            // Held apart, a site is the point itself: "the tips 20 mm apart".
            if want != 0.0 {
                for t in [&mut ta, &mut tb] {
                    if let Some((_, at)) = &t.site {
                        (t.a, t.b, t.r) = (*at, *at, 0.0);
                    }
                }
            }
            let (ca, cb) = seg_closest(ta.a, ta.b, tb.a, tb.b);
            let gap = ca.distance(cb) - ta.r - tb.r;
            let toward = (cb - ca).normalize_or_zero();
            let mut res = vec![gap - want];
            let mut met = ((gap - want) * 1000.0).abs() <= TOUCH_TOL_MM;
            let mut notes = Vec::new();
            for (t, surface) in [(&ta, ca + toward * ta.r), (&tb, cb - toward * tb.r)] {
                if let Some((name, at)) = &t.site {
                    let d = surface - *at;
                    let mm = d.length() * 1000.0;
                    let excess = d * ((mm - SITE_FREE_MM).max(0.0) / mm.max(1e-9));
                    res.extend([excess.x, excess.y, excess.z].map(|x| x * SITE_WEIGHT));
                    met &= mm <= SITE_TOL_MM;
                    if mm >= 1.0 {
                        notes.push(format!("{mm:.1} mm from {name}"));
                    }
                }
            }
            let note = if notes.is_empty() { String::new() } else { format!("contact {}", notes.join(", ")) };
            Ok((res, GoalStatus { label, value: (gap * 10000.0).round() / 10.0, unit: "mm", met, note }))
        }
        (None, Some(finger)) => {
            let at = DVec3::from(goal.at.ok_or(format!("aim {finger}: needs \"at\": [x, y, z]"))?);
            let (base, ray) = s.finger_ray(finger)?;
            let to = (at - base).normalize();
            let c = ray.cross(to) * AIM_WEIGHT;
            let angle = ray.dot(to).clamp(-1.0, 1.0).acos().to_degrees();
            // The cross product vanishes when pointing straight away too; penalize that.
            let back = if ray.dot(to) < 0.0 { AIM_WEIGHT } else { 0.0 };
            Ok((
                vec![c.x, c.y, c.z, back],
                GoalStatus { label: format!("aim {finger} at {:?}", goal.at.unwrap()), value: (angle * 10.0).round() / 10.0, unit: "deg", met: angle <= AIM_TOL_DEG, note: String::new() },
            ))
        }
        // A goal with only `vary`: keep what it varies from passing through anything.
        (None, None) if !goal.vary.is_empty() => {
            let caps: Vec<_> = s.hands.iter().flat_map(|h| &h.capsules).collect();
            let mut deepest = 0.0_f64;
            for (i, a) in caps.iter().enumerate() {
                for b in &caps[i + 1..] {
                    if crate::query::collides(a, b) {
                        deepest = deepest.min(crate::query::seg_dist(a.a, a.b, b.a, b.b) - a.r - b.r);
                    }
                }
            }
            let mm = deepest * 1000.0;
            Ok((Vec::new(), GoalStatus { label: format!("clear {}", goal.vary.join(", ")), value: (mm * 10.0).round() / 10.0, unit: "mm", met: mm >= -CLEAR_TOL_MM, note: String::new() }))
        }
        _ => Err("each goal needs either \"touch\": [a, b], \"aim\": \"<hand>.<digit>\" with \"at\", \"grip\": [a, b] with \"style\", or only \"vary\" (keep those parts from passing through anything)".into()),
    }
}

pub fn statuses(s: &Solved) -> Result<Vec<GoalStatus>, String> {
    s.pose.goals.iter().map(|g| evaluate(g, s).map(|(_, st)| st)).collect()
}
