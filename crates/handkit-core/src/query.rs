//! What an agent checks after every edit, without an image: limits, contacts, collisions,
//! site gaps and aim.

use glam::DVec3;
use serde_json::{Value, json};

use crate::pose::PoseFile;
use crate::rig::{Capsule, Digit, HandGeom, Seg};

/// Surface gaps (mm) at or below this count as touching.
pub const CONTACT_MM: f64 = 3.0;
/// Overlaps deeper than this (mm) count as passing through.
pub const PENETRATION_MM: f64 = 2.0;

pub struct Pair {
    pub a: String,
    pub b: String,
    pub gap_mm: f64,
}

pub struct ProbeHit {
    pub probe: String,
    pub finger: String,
    pub angle_deg: f64,
    pub miss_mm: f64,
}

pub struct Report {
    pub violations: Vec<String>,
    pub coupled: Vec<String>,
    pub contacts: Vec<Pair>,
    pub penetrations: Vec<Pair>,
    pub thumb_nearest: Vec<Pair>,
    pub checks: Vec<Pair>,
    pub probes: Vec<ProbeHit>,
    pub tips: Vec<(String, DVec3)>,
    pub sites: Vec<(String, DVec3)>,
}

pub fn analyze(pose: &PoseFile, hands: &[HandGeom]) -> Result<Report, String> {
    let caps: Vec<&Capsule> = hands.iter().flat_map(|h| &h.capsules).collect();
    let mut contacts = Vec::new();
    let mut penetrations = Vec::new();
    for (i, a) in caps.iter().enumerate() {
        for b in &caps[i + 1..] {
            if !collides(a, b) {
                continue;
            }
            let gap = (seg_dist(a.a, a.b, b.a, b.b) - a.r - b.r) * 1000.0;
            let pair = Pair { a: a.label(), b: b.label(), gap_mm: round1(gap) };
            if gap < -PENETRATION_MM {
                penetrations.push(pair);
            } else if gap <= CONTACT_MM && worth_listing(a, b) {
                contacts.push(pair);
            }
        }
    }
    contacts.sort_by(|x, y| x.gap_mm.total_cmp(&y.gap_mm));
    penetrations.sort_by(|x, y| x.gap_mm.total_cmp(&y.gap_mm));

    // The nearest other part to each thumb tip segment, touching or not.
    let mut thumb_nearest = Vec::new();
    for t in caps.iter().filter(|c| c.digit == Some(Digit::Thumb) && c.seg == Seg::Distal) {
        let best = caps
            .iter()
            .filter(|c| c.digit != Some(Digit::Thumb) || c.side != t.side)
            .filter(|c| c.is_phalanx() || c.side != t.side)
            .map(|c| (c, (seg_dist(t.a, t.b, c.a, c.b) - t.r - c.r) * 1000.0))
            .min_by(|x, y| x.1.total_cmp(&y.1));
        if let Some((c, gap)) = best {
            thumb_nearest.push(Pair { a: t.label(), b: c.label(), gap_mm: round1(gap) });
        }
    }

    // A check names a site (a surface point) or a segment (a capsule, e.g. right.index.middle).
    let target = |name: &str| -> Result<(DVec3, DVec3, f64), String> {
        if let Some(s) = hands.iter().flat_map(|h| &h.sites).find(|s| s.name == name) {
            return Ok((s.pos, s.pos, 0.0));
        }
        if let Some(c) = caps.iter().find(|c| c.label() == name) {
            return Ok((c.a, c.b, c.r));
        }
        Err(format!("unknown site or segment \"{name}\". Sites are <hand>.<digit>.<tip|pad|nail|knuckle>, <hand>.<index|middle|ring|pinky>.<side_radial|side_ulnar|dip_radial|dip_ulnar>, <hand>.thumb.<side_palmar|side_dorsal>, and <hand>.<palm_center|palm_heel|wrist_back>. Segments are <hand>.<digit>.<metacarpal|proximal|middle|distal> (the thumb has metacarpal, proximal, distal) and <hand>.forearm. The hand must be in the file"))
    };
    let mut checks = Vec::new();
    for [a, b] in &pose.checks {
        let ((a0, a1, ra), (b0, b1, rb)) = (target(a)?, target(b)?);
        let gap = (seg_dist(a0, a1, b0, b1) - ra - rb) * 1000.0;
        checks.push(Pair { a: a.clone(), b: b.clone(), gap_mm: round1(gap) });
    }

    let mut probes = Vec::new();
    for (name, p) in &pose.probes {
        let p = DVec3::from(*p);
        for h in hands {
            for d in Digit::ALL {
                let Some(base) = h.capsules.iter().find(|c| c.digit == Some(d) && c.seg == Seg::Proximal) else { continue };
                let tip = h.sites.iter().find(|s| s.name == format!("{}.{}.tip", h.side.name(), d.name())).unwrap().pos;
                let ray = (tip - base.a).normalize();
                let to = p - base.a;
                let angle = ray.dot(to.normalize()).clamp(-1.0, 1.0).acos().to_degrees();
                let miss = (to - ray * to.dot(ray)).length() * 1000.0;
                probes.push(ProbeHit { probe: name.clone(), finger: format!("{}.{}", h.side.name(), d.name()), angle_deg: round1(angle), miss_mm: round1(miss) });
            }
        }
    }
    probes.sort_by(|x, y| x.probe.cmp(&y.probe).then(x.angle_deg.total_cmp(&y.angle_deg)));

    let tips = hands
        .iter()
        .flat_map(|h| &h.sites)
        .filter(|s| s.name.ends_with(".tip"))
        .map(|s| (s.name.clone(), s.pos))
        .collect();

    Ok(Report {
        violations: hands.iter().flat_map(|h| h.violations.clone()).collect(),
        coupled: hands.iter().flat_map(|h| h.coupled.clone()).collect(),
        contacts,
        penetrations,
        thumb_nearest,
        checks,
        probes,
        tips,
        sites: hands.iter().flat_map(|h| &h.sites).map(|s| (s.name.clone(), s.pos)).collect(),
    })
}

/// Pairs that can collide: anything across hands; within a hand, different digits where at
/// least one is a moving segment (a proximal phalanx always touches the palm at its knuckle).
fn collides(a: &Capsule, b: &Capsule) -> bool {
    if a.side != b.side {
        return true;
    }
    if a.part == b.part {
        return false;
    }
    let (fa, fb) = (a.is_phalanx(), b.is_phalanx());
    if !fa && !fb {
        return false;
    }
    !((a.seg == Seg::Proximal && !fb) || (b.seg == Seg::Proximal && !fa))
}

/// Contacts worth reporting: across hands, anything with the thumb, and fingertips on the palm.
/// Neighbouring fingers lying side by side are left out.
fn worth_listing(a: &Capsule, b: &Capsule) -> bool {
    a.side != b.side
        || a.digit == Some(Digit::Thumb)
        || b.digit == Some(Digit::Thumb)
        || (a.seg == Seg::Distal && !b.is_phalanx())
        || (b.seg == Seg::Distal && !a.is_phalanx())
}

fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

fn v3(p: DVec3) -> String {
    format!("[{:.3} {:.3} {:.3}]", p.x, p.y, p.z)
}

impl Report {
    /// `all_sites` lists every site's position instead of only the fingertips.
    pub fn text(&self, all_sites: bool) -> String {
        let mut s = String::new();
        let list = |s: &mut String, title: &str, items: &[String]| {
            if items.is_empty() {
                s.push_str(&format!("{title}: none\n"));
            } else {
                s.push_str(&format!("{title}:\n"));
                for i in items {
                    s.push_str(&format!("  {i}\n"));
                }
            }
        };
        let pairs = |ps: &[Pair]| ps.iter().map(|p| format!("{} ~ {}: {} mm", p.a, p.b, p.gap_mm)).collect::<Vec<_>>();
        list(&mut s, "limit violations", &self.violations);
        list(&mut s, &format!("penetrations (overlap > {PENETRATION_MM} mm)"), &pairs(&self.penetrations));
        list(&mut s, &format!("contacts (gap <= {CONTACT_MM} mm)"), &pairs(&self.contacts));
        list(&mut s, "nearest to each thumb tip", &pairs(&self.thumb_nearest));
        if !self.checks.is_empty() {
            list(&mut s, "checks (site to site)", &pairs(&self.checks));
        }
        if !self.probes.is_empty() {
            let mut lines = Vec::new();
            let mut seen = std::collections::BTreeSet::new();
            for p in &self.probes {
                // Best finger per probe, plus every index finger.
                if seen.insert(p.probe.clone()) || p.finger.ends_with(".index") {
                    lines.push(format!("{}: {} off by {} deg, misses by {} mm", p.probe, p.finger, p.angle_deg, p.miss_mm));
                }
            }
            list(&mut s, "aim (finger line from knuckle through tip)", &lines);
        }
        if !self.coupled.is_empty() {
            list(&mut s, "coupled defaults", &self.coupled);
        }
        let (title, points) = if all_sites { ("sites (m)", &self.sites) } else { ("fingertips (m; --sites for all sites)", &self.tips) };
        list(&mut s, title, &points.iter().map(|(n, p)| format!("{n} {}", v3(*p))).collect::<Vec<_>>());
        s
    }

    pub fn json(&self) -> Value {
        let pairs = |ps: &[Pair]| ps.iter().map(|p| json!({"a": p.a, "b": p.b, "gap_mm": p.gap_mm})).collect::<Vec<_>>();
        json!({
            "joint_limit_violations": self.violations,
            "penetrations": pairs(&self.penetrations),
            "contacts": pairs(&self.contacts),
            "thumb_nearest": pairs(&self.thumb_nearest),
            "checks": pairs(&self.checks),
            "aim": self.probes.iter().map(|p| json!({"probe": p.probe, "finger": p.finger, "angle_deg": p.angle_deg, "miss_mm": p.miss_mm})).collect::<Vec<_>>(),
            "coupled": self.coupled,
            "fingertips_m": self.tips.iter().map(|(n, p)| (n.clone(), json!([p.x, p.y, p.z]))).collect::<serde_json::Map<_, _>>(),
        })
    }
}

/// Closest distance between segments p1-q1 and p2-q2.
pub fn seg_dist(p1: DVec3, q1: DVec3, p2: DVec3, q2: DVec3) -> f64 {
    let (d1, d2, r) = (q1 - p1, q2 - p2, p1 - p2);
    let (a, e, f) = (d1.dot(d1), d2.dot(d2), d2.dot(r));
    let (s, t);
    if a <= 1e-12 && e <= 1e-12 {
        return p1.distance(p2);
    }
    if a <= 1e-12 {
        s = 0.0;
        t = (f / e).clamp(0.0, 1.0);
    } else {
        let c = d1.dot(r);
        if e <= 1e-12 {
            t = 0.0;
            s = (-c / a).clamp(0.0, 1.0);
        } else {
            let b = d1.dot(d2);
            let den = a * e - b * b;
            let s0 = if den != 0.0 { ((b * f - c * e) / den).clamp(0.0, 1.0) } else { 0.0 };
            let t0 = (b * s0 + f) / e;
            if t0 < 0.0 {
                t = 0.0;
                s = (-c / a).clamp(0.0, 1.0);
            } else if t0 > 1.0 {
                t = 1.0;
                s = ((b - c) / a).clamp(0.0, 1.0);
            } else {
                s = s0;
                t = t0;
            }
        }
    }
    (p1 + d1 * s).distance(p2 + d2 * t)
}
