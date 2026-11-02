//! The hand's shape as a signed distance field (meters, negative inside), built from the rig's
//! bind pose. Every primitive belongs to a bone, so the same field gives the skin weights.

use glam::{DAffine3, DVec3, dvec3};
use handkit_core::rig::{Digit, HandGeom, Seg};

pub enum Shape {
    Capsule { a: DVec3, b: DVec3, r: f64 },
    /// A capsule whose radius tapers from `ra` at `a` to `rb` at `b`.
    RoundCone { a: DVec3, b: DVec3, ra: f64, rb: f64 },
    /// An ellipsoid with radii `r` along the axes of `to_local`'s inverse.
    Ellipsoid { to_local: DAffine3, r: DVec3 },
    /// A box with rounded edges: half extents `h` (before rounding) and radius `round`.
    RoundBox { to_local: DAffine3, h: DVec3, round: f64 },
    /// The forearm: elliptical sections along local y from 0 to `len`, radii (x, z) blending
    /// from `r0` to `r1`, with rounded ends.
    Limb { to_local: DAffine3, len: f64, r0: [f64; 2], r1: [f64; 2] },
}

pub struct Prim {
    pub shape: Shape,
    /// Index into the rig's bones.
    pub bone: usize,
    /// Primitives of one group blend into each other with `k`; groups blend into the hand with
    /// the group's own `join`.
    pub group: usize,
}

pub struct Field {
    pub prims: Vec<Prim>,
    /// (k inside the group, k joining the result so far), in the order they are combined.
    pub groups: Vec<(f64, f64)>,
}

impl Shape {
    pub fn dist(&self, p: DVec3) -> f64 {
        match self {
            Shape::Capsule { a, b, r } => {
                let (pa, ba) = (p - *a, *b - *a);
                let t = (pa.dot(ba) / ba.length_squared().max(1e-18)).clamp(0.0, 1.0);
                (pa - ba * t).length() - r
            }
            Shape::RoundCone { a, b, ra, rb } => {
                // Inigo Quilez's exact round cone.
                let ba = *b - *a;
                let l2 = ba.length_squared();
                let rr = ra - rb;
                let a2 = l2 - rr * rr;
                let il2 = 1.0 / l2;
                let pa = p - *a;
                let y = pa.dot(ba);
                let z = y - l2;
                let x2 = (pa * l2 - ba * y).length_squared();
                let (y2, z2) = (y * y * l2, z * z * l2);
                let k = rr.signum() * rr * rr * x2;
                if z.signum() * a2 * z2 > k {
                    (x2 + z2).sqrt() * il2 - rb
                } else if y.signum() * a2 * y2 < k {
                    (x2 + y2).sqrt() * il2 - ra
                } else {
                    ((x2 * a2 * il2).sqrt() + y * rr) * il2 - ra
                }
            }
            Shape::Ellipsoid { to_local, r } => {
                let q = to_local.transform_point3(p);
                let k0 = (q / *r).length();
                let k1 = (q / (*r * *r)).length();
                if k1 < 1e-12 { -r.min_element() } else { k0 * (k0 - 1.0) / k1 }
            }
            Shape::RoundBox { to_local, h, round } => {
                let q = to_local.transform_point3(p).abs() - *h;
                q.max(DVec3::ZERO).length() + q.max_element().min(0.0) - round
            }
            Shape::Limb { to_local, len, r0, r1 } => {
                let q = to_local.transform_point3(p);
                let t = (q.y / len).clamp(0.0, 1.0);
                let (a, b) = (r0[0] + (r1[0] - r0[0]) * t, r0[1] + (r1[1] - r0[1]) * t);
                // Scale the section to a circle of radius b, then it is a capsule along y.
                let s = dvec3(q.x * b / a, q.y.clamp(0.0, *len), q.z);
                let radial = (s.x * s.x + s.z * s.z).sqrt();
                let along = (q.y - s.y).abs();
                (radial * radial + along * along).sqrt() - b
            }
        }
    }
}

/// Polynomial smooth minimum: blends over a band of width `k`.
pub fn smin(a: f64, b: f64, k: f64) -> f64 {
    if k <= 0.0 {
        return a.min(b);
    }
    let h = (k - (a - b).abs()).max(0.0) / k;
    a.min(b) - h * h * k * 0.25
}

impl Field {
    pub fn eval(&self, p: DVec3) -> f64 {
        let mut total = f64::INFINITY;
        for (g, &(k_in, k_join)) in self.groups.iter().enumerate() {
            let mut d = f64::INFINITY;
            for prim in self.prims.iter().filter(|q| q.group == g) {
                d = smin(d, prim.shape.dist(p), k_in);
            }
            total = if total.is_infinite() { d } else { smin(total, d, k_join) };
        }
        total
    }

    pub fn gradient(&self, p: DVec3) -> DVec3 {
        let e = 1e-4;
        dvec3(
            self.eval(p + DVec3::X * e) - self.eval(p - DVec3::X * e),
            self.eval(p + DVec3::Y * e) - self.eval(p - DVec3::Y * e),
            self.eval(p + DVec3::Z * e) - self.eval(p - DVec3::Z * e),
        )
        .normalize_or_zero()
    }
}

fn bone(h: &HandGeom, name: &str) -> usize {
    h.bones.iter().position(|b| b.name == name).unwrap_or_else(|| panic!("no bone {name}"))
}

/// The local frame of a bone at bind, as world -> local.
fn local(h: &HandGeom, name: &str) -> DAffine3 {
    h.bones[bone(h, name)].frame.inverse()
}

/// The field of a right hand in its bind pose (`h` built with the forearm frame at the origin).
pub fn hand_field(h: &HandGeom) -> Field {
    let mut prims = Vec::new();
    let mut groups = Vec::new();
    let hand = bone(h, "hand");
    let hand_local = local(h, "hand");

    // Group 0: the forearm, tapering from the elbow to an oval wrist.
    let wrist_y = h.bones[hand].origin.y;
    prims.push(Prim {
        shape: Shape::Limb { to_local: DAffine3::IDENTITY, len: wrist_y - 0.006, r0: [0.034, 0.030], r1: [0.027, 0.0185] },
        bone: bone(h, "forearm"),
        group: 0,
    });
    groups.push((0.0, 0.0));

    // Group 1: the palm: a rounded slab, the metacarpal heads (knuckles) and the thenar and
    // hypothenar pads, blended softly.
    prims.push(Prim {
        shape: Shape::RoundBox { to_local: DAffine3::from_translation(-dvec3(0.0005, 0.046, -0.0005)) * hand_local, h: dvec3(0.031, 0.036, 0.0035), round: 0.0095 },
        bone: hand,
        group: 1,
    });
    for c in h.capsules.iter().filter(|c| c.seg == Seg::Metacarpal && c.part == "palm") {
        prims.push(Prim { shape: Shape::Capsule { a: c.a, b: c.b, r: c.r }, bone: hand, group: 1 });
    }
    let ellipsoid = |center: DVec3, r: DVec3| Shape::Ellipsoid { to_local: DAffine3::from_translation(-center) * hand_local, r };
    // The thenar pad moves with the thumb's metacarpal, as its muscles do.
    prims.push(Prim { shape: ellipsoid(dvec3(0.021, 0.028, 0.006), dvec3(0.017, 0.024, 0.010)), bone: bone(h, "thumb_cmc"), group: 1 });
    prims.push(Prim { shape: ellipsoid(dvec3(-0.027, 0.034, 0.005), dvec3(0.013, 0.027, 0.009)), bone: hand, group: 1 });
    groups.push((0.006, 0.012));

    // One group per digit: its segments, with a soft fingertip pulp.
    for d in Digit::ALL {
        let g = groups.len();
        let joints: [&str; 3] = if d == Digit::Thumb { ["cmc", "mcp", "ip"] } else { ["mcp", "pip", "dip"] };
        let segs: [Seg; 3] = if d == Digit::Thumb { [Seg::Metacarpal, Seg::Proximal, Seg::Distal] } else { [Seg::Proximal, Seg::Middle, Seg::Distal] };
        let caps: Vec<_> = segs.iter().map(|seg| h.capsules.iter().find(|c| c.digit == Some(d) && c.seg == *seg && c.part != "palm").unwrap()).collect();
        for (i, (seg, joint)) in segs.iter().zip(joints).enumerate() {
            let name = format!("{}_{joint}", d.name());
            let c = caps[i];
            // Taper smoothly into the next segment's radius instead of stepping down at joints.
            let rb = caps.get(i + 1).map(|n| n.r).unwrap_or(c.r * 0.92);
            prims.push(Prim { shape: Shape::RoundCone { a: c.a, b: c.b, ra: c.r, rb }, bone: bone(h, &name), group: g });
            if *seg == Seg::Distal {
                let b = &h.bones[bone(h, &name)];
                let (l, r) = (b.len, c.r);
                let center = b.frame.transform_point3(dvec3(0.0, 0.6 * l, 0.22 * r));
                let to_local = DAffine3::from_translation(-center) * b.frame.inverse();
                prims.push(Prim { shape: Shape::Ellipsoid { to_local, r: dvec3(0.9 * r, 0.55 * l, 0.92 * r) }, bone: bone(h, &name), group: g });
            }
        }
        // The thumb's web and the palm's knuckles blend wider than finger joints.
        groups.push((0.003, if d == Digit::Thumb { 0.014 } else { 0.006 }));
    }
    Field { prims, groups }
}

/// How strongly a point belongs to each bone, falling off with distance beyond the nearest
/// primitive (`band`, meters, wider at the wrist and thumb base).
/// Wider blends where the skin stretches over big moves: the wrist and the base of the thumb.
fn band_between(h: &HandGeom, a: usize, b: usize, band: f64) -> f64 {
    let names = [h.bones[a].name.as_str(), h.bones[b].name.as_str()];
    if names.contains(&"forearm") {
        band * 4.0
    } else if names.contains(&"thumb_cmc") {
        band * 3.0
    } else {
        band
    }
}

pub fn weights(field: &Field, h: &HandGeom, p: DVec3, band: f64) -> Vec<(usize, f64)> {
    let n = h.bones.len();
    let mut per_bone = vec![f64::INFINITY; n];
    for prim in &field.prims {
        let d = prim.shape.dist(p);
        if d < per_bone[prim.bone] {
            per_bone[prim.bone] = d;
        }
    }
    let nearest = (0..n).min_by(|&a, &b| per_bone[a].total_cmp(&per_bone[b])).unwrap();
    let near = per_bone[nearest];
    // Blend freely within a digit and with the palm and forearm, never between two digits, so
    // neighbouring fingers do not drag each other.
    let digit = |b: usize| h.bones[b].name.split('_').next().filter(|d| ["thumb", "index", "middle", "ring", "pinky"].contains(d));
    let related = |b: usize| digit(b).is_none() || digit(nearest).is_none() || digit(b) == digit(nearest);
    let mut out: Vec<(usize, f64)> = (0..n)
        .filter(|&b| related(b) && per_bone[b].is_finite())
        .map(|b| (b, (-(per_bone[b] - near) / band_between(h, nearest, b, band)).exp()))
        .filter(|(_, w)| *w > 0.02)
        .collect();
    out.sort_by(|a, b| b.1.total_cmp(&a.1));
    out.truncate(4);
    let sum: f64 = out.iter().map(|x| x.1).sum();
    out.iter_mut().for_each(|x| x.1 /= sum);
    out
}
