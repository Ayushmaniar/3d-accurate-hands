//! The hand's shape as a signed distance field (meters, negative inside), built from the rig's
//! bind pose. Every primitive belongs to a bone, so the same field gives the skin weights.
//!
//! The anatomy, in short: fingers are wider than deep, flatter on the back, with a fleshy pad
//! on the palm side of each segment and a bony knuckle on the back of each joint; the last
//! segment carries a raised nail plate on a flattened nail bed. The palm is a thin slab with a
//! thick heel, thenar and hypothenar pads, pads under the knuckles, metacarpal heads and faint
//! extensor tendons on the back; webs join the finger bases well above the knuckles. The
//! forearm is round near the elbow and flat at the wrist, with the ulna's head on the pinky
//! side. Shallow creases (palm lines, finger flexion creases, wrist) are carved last.

use glam::{DAffine3, DMat3, DVec2, DVec3, dvec2, dvec3};
use handkit_core::rig::{Digit, HandGeom, Seg};

/// A capsule along local y from 0 to `len` whose section is an ellipse (a superellipse with
/// exponent `n_back` on the back, flatter for n > 2): half width `w`, depth `front` toward the
/// palm (+z) and `back` toward the back of the hand, each blending from the start to the end;
/// the ends are rounded over `caps` along y.
#[derive(Clone, Copy, Debug)]
pub struct Phalanx {
    pub to_local: DAffine3,
    pub len: f64,
    pub w: [f64; 2],
    pub front: [f64; 2],
    pub back: [f64; 2],
    pub caps: [f64; 2],
    pub n_back: f64,
}

impl Phalanx {
    pub fn dist_local(&self, q: DVec3) -> f64 {
        let t = (q.y / self.len).clamp(0.0, 1.0);
        let lerp = |v: [f64; 2]| v[0] + (v[1] - v[0]) * t;
        let a = lerp(self.w);
        let (c, n) = if q.z > 0.0 { (lerp(self.front), 2.0) } else { (lerp(self.back), self.n_back) };
        let (along, cap) = if q.y < 0.0 {
            (-q.y, self.caps[0])
        } else if q.y > self.len {
            (q.y - self.len, self.caps[1])
        } else {
            (0.0, self.caps[0].max(self.caps[1]))
        };
        section_dist(q.x, q.z, along, a, c, cap, n)
    }
}

/// Distance (first order) to the surface rho = 1, where rho is the superellipse norm of
/// (x / a, z / c) combined with the distance past the end (`along / cap`).
fn section_dist(x: f64, z: f64, along: f64, a: f64, c: f64, cap: f64, n: f64) -> f64 {
    let (ux, uz, us) = (x.abs() / a, z.abs() / c, along / cap);
    let (r2, gx, gz) = if n == 2.0 {
        let r2 = (ux * ux + uz * uz).sqrt();
        if r2 > 1e-12 { (r2, ux / r2 / a, uz / r2 / c) } else { (0.0, 0.0, 0.0) }
    } else {
        let (px, pz) = (ux.powf(n), uz.powf(n));
        let r2 = (px + pz).powf(1.0 / n);
        if r2 > 1e-12 {
            let k = r2.powf(1.0 - n);
            (r2, k * px / ux.max(1e-12) / a, k * pz / uz.max(1e-12) / c)
        } else {
            (0.0, 0.0, 0.0)
        }
    };
    let rho = (r2 * r2 + us * us).sqrt();
    let (gx, gz, gs) = if rho > 1e-12 { (gx * r2 / rho, gz * r2 / rho, us / rho / cap) } else { (0.0, 0.0, 0.0) };
    let g = (gx * gx + gz * gz + gs * gs).sqrt().max(1.0 / a.max(c).max(cap));
    (rho - 1.0) / g
}

pub type Func = Box<dyn Fn(DVec3) -> f64 + Send + Sync>;

pub enum Shape {
    Capsule { a: DVec3, b: DVec3, r: f64 },
    /// A capsule whose radius tapers from `ra` at `a` to `rb` at `b`.
    RoundCone { a: DVec3, b: DVec3, ra: f64, rb: f64 },
    /// An ellipsoid with radii `r` along the axes of `to_local`'s inverse.
    Ellipsoid { to_local: DAffine3, r: DVec3 },
    /// A bone's flesh in the bone's frame.
    Phalanx(Phalanx),
    /// The forearm: superelliptic sections along local y from 0 to `len`; `keys` are
    /// (t, half width, half depth, exponent) at fractions t of the length, interpolated smoothly.
    Limb { to_local: DAffine3, len: f64, keys: Vec<[f64; 4]> },
    /// Anything else, with a bounding sphere (center, radius) for culling.
    Func { f: Func, bound: (DVec3, f64) },
}

pub struct Prim {
    pub shape: Shape,
    /// Index into the rig's bones.
    pub bone: usize,
    /// Primitives of one group blend into each other in order, each with its own `k`; groups
    /// blend into the hand with the group's own `join`.
    pub group: usize,
    pub k: f64,
}

pub struct Group {
    pub join: f64,
    /// A sphere (center, radius) containing the group's surface, for culling.
    pub bound: (DVec3, f64),
}

/// A groove carved into the skin: a chain of tube segments lying just above the surface.
pub struct Cut {
    pub segs: Vec<(DVec3, DVec3)>,
    pub r: f64,
    pub k: f64,
    pub bound: (DVec3, f64),
}

/// The last segment of a digit with its nail plate: the phalanx's surface lifted by `lift`
/// inside a rounded rectangle on its back (local x within +-`hx`, y from `y0` to `y1`), with a
/// soft edge, so the plate is one smooth surface with the skin around it.
#[derive(Clone, Copy, Debug)]
pub struct NailPlate {
    pub ph: Phalanx,
    pub hx: f64,
    pub y0: f64,
    pub y1: f64,
    pub round: f64,
    pub lift: f64,
    /// Only the back: local z below this.
    pub z_max: f64,
}

impl NailPlate {
    /// Signed distance in the plate's plane to its outline.
    fn outline(&self, q: DVec3) -> f64 {
        let c = dvec2(0.0, 0.5 * (self.y0 + self.y1));
        let h = dvec2(self.hx, 0.5 * (self.y1 - self.y0)) - DVec2::splat(self.round);
        let d = (dvec2(q.x, q.y) - c).abs() - h;
        d.max(DVec2::ZERO).length() + d.x.max(d.y).min(0.0) - self.round
    }

    /// How much of the plate is at a point: 1 inside the outline on the back, 0 outside.
    fn plate(&self, q: DVec3, soft: f64) -> f64 {
        smoothstep(soft, -soft, self.outline(q)) * smoothstep(self.z_max + 0.001, self.z_max - 0.001, q.z)
    }

    pub fn dist(&self, p: DVec3) -> f64 {
        let q = self.ph.to_local.transform_point3(p);
        // The plate rises out of the cuticle over the first part of its length.
        let rise = 0.4 + 0.6 * smoothstep(self.y0, self.y0 + 0.35 * (self.y1 - self.y0), q.y);
        // A fine groove where the skin folds over the plate's root and sides (not its free edge).
        let o = self.outline(q);
        let folds = smoothstep(self.y1 - 0.2 * (self.y1 - self.y0), self.y1 - 0.45 * (self.y1 - self.y0), q.y)
            * smoothstep(self.z_max + 0.001, self.z_max - 0.001, q.z);
        let groove = 0.00025 * folds * (-(o / 0.0005).powi(2)).exp();
        self.ph.dist_local(q) - self.lift * rise * self.plate(q, 0.0005) + groove
    }

    /// 1 on the plate, 0 on the skin, soft over about a millimeter.
    pub fn mask(&self, p: DVec3) -> f64 {
        self.plate(self.ph.to_local.transform_point3(p), 0.0005)
    }
}

pub struct Field {
    pub prims: Vec<Prim>,
    pub groups: Vec<Group>,
    pub cuts: Vec<Cut>,
    pub nails: Vec<NailPlate>,
}

fn sphere_of(points: &[(DVec3, f64)]) -> (DVec3, f64) {
    let c = points.iter().map(|x| x.0).sum::<DVec3>() / points.len() as f64;
    let r = points.iter().map(|(p, r)| (*p - c).length() + r).fold(0.0, f64::max);
    (c, r)
}

/// Smooth interpolation of the forearm's profile keys.
fn profile(keys: &[[f64; 4]], t: f64) -> [f64; 3] {
    let n = keys.len();
    let i = keys.iter().rposition(|k| k[0] <= t).unwrap_or(0).min(n - 2);
    let (k0, k1) = (keys[i], keys[i + 1]);
    let h = k1[0] - k0[0];
    let s = ((t - k0[0]) / h).clamp(0.0, 1.0);
    // Flat at both ends, so the rounded ends meet the sides without a kink.
    let slope = |j: usize, c: usize| {
        if j == 0 || j == n - 1 {
            return 0.0;
        }
        (keys[j + 1][c] - keys[j - 1][c]) / (keys[j + 1][0] - keys[j - 1][0])
    };
    let (h00, h10, h01, h11) = (2.0 * s * s * s - 3.0 * s * s + 1.0, s * s * s - 2.0 * s * s + s, -2.0 * s * s * s + 3.0 * s * s, s * s * s - s * s);
    std::array::from_fn(|j| {
        let c = j + 1;
        h00 * k0[c] + h10 * h * slope(i, c) + h01 * k1[c] + h11 * h * slope(i + 1, c)
    })
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
            Shape::Phalanx(ph) => ph.dist_local(ph.to_local.transform_point3(p)),
            Shape::Limb { to_local, len, keys } => {
                let q = to_local.transform_point3(p);
                let [a, c, n] = profile(keys, (q.y / len).clamp(0.0, 1.0));
                let along = if q.y < 0.0 { -q.y } else { (q.y - len).max(0.0) };
                section_dist(q.x, q.z, along, a, c, c, n)
            }
            Shape::Func { f, .. } => f(p),
        }
    }

    pub fn bound(&self) -> (DVec3, f64) {
        match self {
            Shape::Capsule { a, b, r } => ((*a + *b) * 0.5, (*a - *b).length() * 0.5 + r),
            Shape::RoundCone { a, b, ra, rb } => ((*a + *b) * 0.5, (*a - *b).length() * 0.5 + ra.max(*rb)),
            Shape::Ellipsoid { to_local, r } => (to_local.inverse().translation, r.max_element()),
            Shape::Phalanx(ph) => {
                let c = ph.to_local.inverse().transform_point3(dvec3(0.0, 0.5 * ph.len, 0.0));
                let section = ph.w.iter().chain(&ph.front).chain(&ph.back).fold(0.0f64, |m, x| m.max(*x));
                (c, 0.5 * ph.len + ph.caps[0].max(ph.caps[1]) + section)
            }
            Shape::Limb { to_local, len, keys } => {
                let c = to_local.inverse().transform_point3(dvec3(0.0, 0.5 * len, 0.0));
                let r = keys.iter().fold(0.0f64, |m, k| m.max(k[1]).max(k[2]));
                (c, 0.5 * len + r)
            }
            Shape::Func { bound, .. } => *bound,
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

pub fn smax(a: f64, b: f64, k: f64) -> f64 {
    -smin(-a, -b, k)
}

impl Field {
    /// The skin without the creases.
    pub fn eval_body(&self, p: DVec3) -> f64 {
        let mut total = f64::INFINITY;
        for (g, group) in self.groups.iter().enumerate() {
            // A group farther than the blend band from the surface so far changes nothing.
            if total.is_finite() && (p - group.bound.0).length() - group.bound.1 > total + group.join {
                continue;
            }
            let mut d = f64::INFINITY;
            for prim in self.prims.iter().filter(|q| q.group == g) {
                d = smin(d, prim.shape.dist(p), prim.k);
            }
            total = if total.is_infinite() { d } else { smin(total, d, group.join) };
        }
        total
    }

    pub fn eval(&self, p: DVec3) -> f64 {
        let mut d = self.eval_body(p);
        for cut in &self.cuts {
            if (p - cut.bound.0).length() - cut.bound.1 > cut.k - d {
                continue;
            }
            let tube = cut.segs.iter().map(|(a, b)| seg_dist(p, *a, *b)).fold(f64::INFINITY, f64::min) - cut.r;
            d = smax(d, -tube, cut.k);
        }
        d
    }

    pub fn gradient(&self, p: DVec3) -> DVec3 {
        grad(|x| self.eval(x), p)
    }

    /// 1 on the nail plates, 0 elsewhere.
    pub fn nail_mask(&self, p: DVec3) -> f64 {
        self.nails.iter().map(|n| n.mask(p)).fold(0.0, f64::max)
    }
}

fn grad(f: impl Fn(DVec3) -> f64, p: DVec3) -> DVec3 {
    let e = 1e-4;
    dvec3(
        f(p + DVec3::X * e) - f(p - DVec3::X * e),
        f(p + DVec3::Y * e) - f(p - DVec3::Y * e),
        f(p + DVec3::Z * e) - f(p - DVec3::Z * e),
    )
    .normalize_or_zero()
}

fn seg_dist(p: DVec3, a: DVec3, b: DVec3) -> f64 {
    let (pa, ba) = (p - a, b - a);
    let t = (pa.dot(ba) / ba.length_squared().max(1e-18)).clamp(0.0, 1.0);
    (pa - ba * t).length()
}

fn bone(h: &HandGeom, name: &str) -> usize {
    h.bones.iter().position(|b| b.name == name).unwrap_or_else(|| panic!("no bone {name}"))
}

/// The local frame of a bone at bind, as world -> local.
fn local(h: &HandGeom, name: &str) -> DAffine3 {
    h.bones[bone(h, name)].frame.inverse()
}

fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Signed distance to a simple polygon (Inigo Quilez).
fn polygon(p: DVec2, v: &[DVec2]) -> f64 {
    let mut d = (p - v[0]).length_squared();
    let mut s = 1.0;
    let mut j = v.len() - 1;
    for i in 0..v.len() {
        let e = v[j] - v[i];
        let w = p - v[i];
        let b = w - e * (w.dot(e) / e.length_squared()).clamp(0.0, 1.0);
        d = d.min(b.length_squared());
        let c = (p.y >= v[i].y, p.y < v[j].y, e.x * w.y > e.y * w.x);
        if (c.0 && c.1 && c.2) || (!c.0 && !c.1 && !c.2) {
            s = -s;
        }
        j = i;
    }
    s * d.sqrt()
}

/// An ellipsoid with its axes given in the world (`axes` are unit, `r` the radii along them).
fn ellipsoid(center: DVec3, axes: [DVec3; 3], r: DVec3) -> Shape {
    let to_world = DAffine3::from_mat3_translation(DMat3::from_cols(axes[0], axes[1], axes[2]), center);
    Shape::Ellipsoid { to_local: to_world.inverse(), r }
}

/// Catmull-Rom through `pts`, sampled about every `step` meters.
fn spline(pts: &[DVec2], step: f64) -> Vec<DVec2> {
    let n = pts.len();
    let mut out = vec![pts[0]];
    for i in 0..n - 1 {
        let (p0, p1, p2, p3) = (pts[i.saturating_sub(1)], pts[i], pts[i + 1], pts[(i + 2).min(n - 1)]);
        let m = ((p2 - p1).length() / step).ceil().max(1.0) as usize;
        for s in 1..=m {
            let t = s as f64 / m as f64;
            let (t2, t3) = (t * t, t * t * t);
            out.push(0.5 * ((2.0 * p1) + (p2 - p0) * t + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2 + (3.0 * p1 - p0 - 3.0 * p2 + p3) * t3));
        }
    }
    out
}

/// The palm's dorsal and palmar surfaces as heights along the hand's z (toward the palm), in
/// the hand's frame: the back arches across and carries faint extensor tendons fanning from the
/// wrist to the knuckles; the palm side is thin in the middle and thicker toward the heel.
#[derive(Clone)]
struct PalmHeights {
    tendons: Vec<(DVec2, DVec2)>,
    /// The knuckles' (x, y), index to pinky, for the pads under them.
    knuckles: Vec<DVec2>,
}

impl PalmHeights {
    fn back(&self, x: f64, y: f64) -> f64 {
        let mut z = -0.0126 + 0.0028 * (x / 0.037).powi(2) - 0.0020 * smoothstep(0.05, -0.008, y);
        for (a, b) in &self.tendons {
            let (pa, ba) = (dvec2(x, y) - *a, *b - *a);
            let t = pa.dot(ba) / ba.length_squared();
            let d = (pa - ba * t.clamp(0.0, 1.0)).length();
            // Barely there on a relaxed hand, gone before the knuckles.
            let fade = smoothstep(0.0, 0.35, t) * smoothstep(0.85, 0.55, t);
            z -= 0.00018 * fade * (-(d / 0.0032).powi(2)).exp();
        }
        z
    }

    fn front(&self, x: f64, y: f64) -> f64 {
        // The line of the knuckles, interpolated across the palm.
        let k = &self.knuckles;
        let i = k.iter().position(|m| m.x < x).unwrap_or(k.len()).clamp(1, k.len() - 1);
        let t = ((x - k[i - 1].x) / (k[i].x - k[i - 1].x)).clamp(0.0, 1.0);
        let ky = k[i - 1].y + (k[i].y - k[i - 1].y) * t;
        let pads = 0.0017 * (-((y - (ky - 0.007)) / 0.008).powi(2)).exp();
        0.0112 + 0.0028 * smoothstep(0.035, -0.008, y) + pads
    }
}

/// The field of a right hand in its bind pose (`h` built with the forearm frame at the origin).
pub fn hand_field(h: &HandGeom) -> Field {
    let mut prims: Vec<Prim> = Vec::new();
    let mut groups: Vec<f64> = Vec::new();
    let hand = bone(h, "hand");
    let hand_local = local(h, "hand");
    let hand_world = hand_local.inverse();
    let hw = |x: f64, y: f64, z: f64| hand_world.transform_point3(dvec3(x, y, z));
    let hv = |v: DVec3| hand_world.transform_vector3(v).normalize();
    let forearm = bone(h, "forearm");

    // Group 0: the forearm, round and muscular near the elbow, flat and narrow at the wrist,
    // with the head of the ulna on the back of the pinky side.
    let wrist_y = h.bones[hand].origin.y;
    prims.push(Prim {
        shape: Shape::Limb {
            to_local: DAffine3::IDENTITY,
            len: wrist_y + 0.006,
            keys: vec![
                [0.0, 0.036, 0.032, 2.0],
                [0.22, 0.039, 0.0335, 2.0],
                [0.5, 0.0355, 0.0275, 2.1],
                [0.8, 0.031, 0.0212, 2.3],
                [1.0, 0.0265, 0.0158, 2.6],
            ],
        },
        bone: forearm,
        group: 0,
        k: 0.0,
    });
    prims.push(Prim {
        shape: ellipsoid(hw(-0.0215, -0.013, -0.0075), [hv(DVec3::X), hv(DVec3::Y), hv(DVec3::Z)], dvec3(0.0075, 0.009, 0.0075)),
        bone: forearm,
        group: 0,
        k: 0.006,
    });
    groups.push(0.0);

    // Group 1: the palm.
    let digit_mcp = |d: Digit| h.bones[bone(h, &format!("{}_mcp", d.name()))].origin;
    let mcp_local: Vec<DVec3> = [Digit::Index, Digit::Middle, Digit::Ring, Digit::Pinky].iter().map(|d| hand_local.transform_point3(digit_mcp(*d))).collect();
    let heights = PalmHeights {
        tendons: mcp_local.iter().enumerate().map(|(i, m)| (dvec2(0.004 - 0.003 * i as f64, 0.004), dvec2(m.x, m.y - 0.008))).collect(),
        knuckles: mcp_local.iter().map(|m| dvec2(m.x, m.y)).collect(),
    };
    let outline: Vec<DVec2> = vec![
        dvec2(-0.022, -0.008),
        dvec2(0.022, -0.008),
        dvec2(0.0335, 0.045),
        dvec2(0.0385, 0.080),
        dvec2(mcp_local[0].x, mcp_local[0].y + 0.004),
        dvec2(mcp_local[1].x, mcp_local[1].y + 0.004),
        dvec2(mcp_local[2].x, mcp_local[2].y + 0.004),
        dvec2(mcp_local[3].x, mcp_local[3].y + 0.004),
        dvec2(-0.0365, 0.070),
        dvec2(-0.0355, 0.040),
    ];
    let palm_round = 0.0075;
    {
        let heights = heights.clone();
        let outline = outline.clone();
        let center = hw(0.0, 0.045, 0.0);
        prims.push(Prim {
            shape: Shape::Func {
                f: Box::new(move |p| {
                    let q = hand_local.transform_point3(p);
                    let d2 = polygon(dvec2(q.x, q.y), &outline);
                    let dz = (q.z - heights.front(q.x, q.y)).max(heights.back(q.x, q.y) - q.z);
                    let e = dvec2(d2 + palm_round, dz + palm_round);
                    e.max(DVec2::ZERO).length() + e.x.max(e.y).min(0.0) - palm_round
                }),
                bound: (center, 0.075),
            },
            bone: hand,
            group: 1,
            k: 0.0,
        });
    }
    // Metacarpal heads: the knuckles on the back, a little proud of the back of the hand.
    for (i, m) in mcp_local.iter().enumerate() {
        let r = dvec3([0.0078, 0.0086, 0.0082, 0.0074][i], 0.009, 0.0078);
        let top = heights.back(m.x, m.y - 0.003) - [0.0012, 0.0015, 0.0012, 0.0008][i];
        prims.push(Prim { shape: ellipsoid(hw(m.x, m.y - 0.003, top + r.z), [hv(DVec3::X), hv(DVec3::Y), hv(DVec3::Z)], r), bone: hand, group: 1, k: 0.006 });
    }
    // The hypothenar pad along the pinky side of the heel.
    let axis = dvec3(-0.12, 1.0, 0.0).normalize();
    prims.push(Prim {
        shape: ellipsoid(hw(-0.0245, 0.034, 0.0080), [hv(DVec3::Z.cross(axis)), hv(axis), hv(DVec3::Z)], dvec3(0.0125, 0.030, 0.0084)),
        bone: hand,
        group: 1,
        k: 0.008,
    });
    // The thenar pad: its root in the heel stays with the hand...
    let axis = dvec3(0.56, 0.83, 0.0).normalize();
    prims.push(Prim {
        shape: ellipsoid(hw(0.0185, 0.030, 0.0090), [hv(DVec3::Z.cross(axis)), hv(axis), hv(DVec3::Z)], dvec3(0.0155, 0.026, 0.0100)),
        bone: hand,
        group: 1,
        k: 0.008,
    });
    // ...and its bulk runs along the palm side of the thumb's metacarpal, moving with it.
    {
        let bi = bone(h, "thumb_cmc");
        let b = &h.bones[bi];
        let axes = [b.frame.transform_vector3(DVec3::X), b.frame.transform_vector3(DVec3::Y), b.frame.transform_vector3(DVec3::Z)];
        let center = b.frame.transform_point3(dvec3(0.0035, 0.56 * b.len, 0.004));
        prims.push(Prim { shape: ellipsoid(center, axes, dvec3(0.0105, 0.017, 0.0125)), bone: bi, group: 1, k: 0.008 });
    }
    // Webs between the finger bases, reaching well past the knuckles on the palm side. Each
    // half moves with its finger, as the skin of the web does.
    let fingers = [Digit::Index, Digit::Middle, Digit::Ring, Digit::Pinky];
    for pair in fingers.windows(2) {
        let point = |d: Digit| {
            let bi = bone(h, &format!("{}_mcp", d.name()));
            let b = &h.bones[bi];
            let c = h.capsules.iter().find(|c| c.digit == Some(d) && c.seg == Seg::Proximal).unwrap();
            (b.frame.transform_point3(dvec3(0.0, 0.2 * b.len, 0.3 * c.r)), c.r, bi)
        };
        let ((a, ra, ba), (b, rb, bb)) = (point(pair[0]), point(pair[1]));
        let (mid, r) = ((a + b) * 0.5, 0.6 * 0.5 * (ra + rb));
        prims.push(Prim { shape: Shape::Capsule { a, b: mid, r }, bone: ba, group: 1, k: 0.004 });
        prims.push(Prim { shape: Shape::Capsule { a: mid, b, r }, bone: bb, group: 1, k: 0.004 });
    }
    // The web between thumb and index (the first dorsal interosseous), thinning toward the thumb.
    {
        let t = &h.bones[bone(h, "thumb_mcp")];
        let a = hw(0.029, 0.060, -0.002);
        let b = t.frame.transform_point3(dvec3(0.0, 0.1 * t.len, 0.0));
        prims.push(Prim { shape: Shape::RoundCone { a, b, ra: 0.0085, rb: 0.0062 }, bone: hand, group: 1, k: 0.008 });
    }
    groups.push(0.008);

    // One group per digit.
    let mut nails = Vec::new();
    for d in Digit::ALL {
        let g = groups.len();
        let thumb = d == Digit::Thumb;
        let joints: [&str; 3] = if thumb { ["cmc", "mcp", "ip"] } else { ["mcp", "pip", "dip"] };
        let segs: [Seg; 3] = if thumb { [Seg::Metacarpal, Seg::Proximal, Seg::Distal] } else { [Seg::Proximal, Seg::Middle, Seg::Distal] };
        let caps: Vec<_> = segs.iter().map(|seg| h.capsules.iter().find(|c| c.digit == Some(d) && c.seg == *seg && c.part != "palm").unwrap()).collect();
        for i in 0..3 {
            let bi = bone(h, &format!("{}_{}", d.name(), joints[i]));
            let b = &h.bones[bi];
            let to_local = b.frame.inverse();
            let at = |x: f64, y: f64, z: f64| b.frame.transform_point3(dvec3(x, y, z));
            let axes = [b.frame.transform_vector3(DVec3::X), b.frame.transform_vector3(DVec3::Y), b.frame.transform_vector3(DVec3::Z)];
            let (l, r) = (b.len, caps[i].r);
            let rn = caps.get(i + 1).map(|n| n.r).unwrap_or(r);
            let distal = i == 2;
            let ph = if thumb && i == 0 {
                Phalanx { to_local, len: l, w: [r, rn], front: [0.92 * r, 0.9 * rn], back: [0.9 * r, 0.86 * rn], caps: [0.6 * r, rn], n_back: 2.0 }
            } else if distal {
                Phalanx { to_local, len: l, w: [r + 0.0004, 0.92 * r + 0.0004], front: [0.88 * r, 0.76 * r], back: [0.72 * r, 0.62 * r], caps: [r, 0.97 * r], n_back: 2.6 }
            } else {
                let back = if i == 0 && !thumb { [0.84, 0.80] } else { [0.80, 0.74] };
                Phalanx { to_local, len: l, w: [r + 0.0004, rn + 0.0004], front: [0.9 * r, 0.88 * rn], back: [back[0] * r, back[1] * rn], caps: [r, rn], n_back: 2.4 }
            };
            let nail = distal.then(|| NailPlate {
                ph,
                hx: if thumb { 0.74 * r } else { 0.72 * r },
                y0: if thumb { 0.44 * l } else { 0.46 * l },
                y1: l + 0.55 * r,
                round: if thumb { 0.6 * r } else { 0.56 * r },
                lift: 0.00045,
                z_max: -0.3 * r,
            });
            let shape = match nail {
                Some(nail) => {
                    nails.push(nail);
                    Shape::Func { f: Box::new(move |p| nail.dist(p)), bound: Shape::Phalanx(ph).bound() }
                }
                None => Shape::Phalanx(ph),
            };
            prims.push(Prim { shape, bone: bi, group: g, k: 0.0 });
            // The fleshy pad on the palm side of the segment, reaching the capsule's radius.
            if !(thumb && i == 0) {
                let (yc, ry, rx, rz) = match (distal, i, thumb) {
                    (true, _, true) => (0.56 * l, 0.52 * l, 0.92 * r, 0.52 * r),
                    (true, _, false) => (0.58 * l, 0.5 * l, 0.88 * r, 0.5 * r),
                    (_, 0, false) => (0.6 * l, 0.38 * l, 0.88 * r, 0.5 * r),
                    _ => (0.52 * l, 0.44 * l, 0.88 * r, 0.5 * r),
                };
                prims.push(Prim { shape: ellipsoid(at(0.0, yc, r - rz), axes, dvec3(rx, ry, rz)), bone: bi, group: g, k: 0.004 });
            }
            // The bony knuckle on the back of the joint at the segment's end (the bone's head).
            if !distal {
                let (back_end, bump, size) = if thumb && i == 0 { (0.86, 0.03, 0.5) } else if i == 0 { (0.80, 0.08, 0.68) } else { (0.74, 0.05, 0.56) };
                let rz = 0.45 * rn;
                let center = at(0.0, l - 0.1 * rn, -(back_end + bump) * rn + rz);
                prims.push(Prim { shape: ellipsoid(center, axes, dvec3(size * rn, 0.42 * rn, rz)), bone: bi, group: g, k: 0.003 });
            }
        }
        // The thumb's web and the palm's knuckles blend wider than finger joints.
        groups.push(if thumb { 0.014 } else { 0.006 });
    }

    let groups: Vec<Group> = groups
        .iter()
        .enumerate()
        .map(|(g, &join)| {
            let spheres: Vec<_> = prims.iter().filter(|p| p.group == g).map(|p| p.shape.bound()).collect();
            let (c, r) = sphere_of(&spheres);
            // Approximate distances may overshoot a little far away: keep a margin.
            Group { join, bound: (c, r * 1.1 + 0.004) }
        })
        .collect();
    let mut field = Field { prims, groups, cuts: Vec::new(), nails };
    field.cuts = creases(h, &field);
    field
}

/// Where a ray from `from` along `dir` first meets the skin, with the outward normal there.
fn hit(field: &Field, from: DVec3, dir: DVec3) -> Option<(DVec3, DVec3)> {
    let mut t = 0.0;
    for _ in 0..200 {
        let p = from + dir * t;
        let d = field.eval_body(p);
        if d.abs() < 1e-6 {
            return Some((p, grad(|x| field.eval_body(x), p)));
        }
        t += d.max(1e-6) * 0.9;
        if t > 0.08 {
            break;
        }
    }
    None
}

/// A crease along points (in a frame's local x, y), found by casting along the frame's -z from
/// above the palm side: a tube of radius `r` sunk `depth` into the skin, shallower at the ends.
fn crease(field: &Field, frame: DAffine3, pts: &[DVec2], r: f64, depth: f64, k: f64) -> Option<Cut> {
    let samples = spline(pts, 0.0012);
    let n = samples.len();
    let dir = frame.transform_vector3(DVec3::NEG_Z).normalize();
    let mut centers = Vec::new();
    for (i, s) in samples.iter().enumerate() {
        let Some((p, normal)) = hit(field, frame.transform_point3(dvec3(s.x, s.y, 0.04)), dir) else { continue };
        let u = i as f64 / (n - 1) as f64;
        let fade = (u * (1.0 - u) * 4.0).min(1.0).sqrt();
        centers.push(p + normal * (r - depth * fade));
    }
    if centers.len() < 2 {
        return None;
    }
    let segs: Vec<_> = centers.windows(2).map(|w| (w[0], w[1])).collect();
    let bound = sphere_of(&centers.iter().map(|c| (*c, r)).collect::<Vec<_>>());
    Some(Cut { segs, r, k, bound: (bound.0, bound.1 + 0.001) })
}

/// The palm lines, the wrist crease and each finger's flexion creases.
fn creases(h: &HandGeom, field: &Field) -> Vec<Cut> {
    let hand = h.bones[bone(h, "hand")].frame;
    let v = |p: &[[f64; 2]]| p.iter().map(|q| dvec2(q[0], q[1])).collect::<Vec<_>>();
    let mut cuts = Vec::new();
    let palm: [(Vec<DVec2>, f64); 4] = [
        // Distal transverse (heart line), from the pinky edge to between index and middle.
        (v(&[[-0.040, 0.061], [-0.026, 0.066], [-0.010, 0.071], [0.004, 0.077], [0.016, 0.084]]), 0.00014),
        // Proximal transverse (head line), from the index side across the middle of the palm.
        (v(&[[0.037, 0.070], [0.024, 0.064], [0.008, 0.058], [-0.008, 0.052], [-0.022, 0.046]]), 0.00014),
        // Thenar crease (life line), around the thenar pad.
        (v(&[[0.036, 0.071], [0.024, 0.060], [0.014, 0.046], [0.009, 0.030], [0.010, 0.014], [0.013, 0.002]]), 0.00018),
        // The wrist creases.
        (v(&[[-0.020, -0.003], [-0.008, -0.005], [0.008, -0.005], [0.018, -0.003]]), 0.0001),
    ];
    for (pts, depth) in palm.iter() {
        cuts.extend(crease(field, hand, pts, 0.0045, *depth, 0.0015));
    }
    // Finger flexion creases: at the base of each finger, and under the pip and dip joints.
    for d in Digit::ALL {
        let thumb = d == Digit::Thumb;
        let names: &[(&str, f64, f64)] =
            if thumb { &[("thumb_mcp", 0.0, 0.0003), ("thumb_ip", 0.0, 0.0003)] } else { &[("_mcp", 0.27, 0.00022), ("_pip", -0.02, 0.00026), ("_dip", -0.02, 0.0002)] };
        for (suffix, at, depth) in names {
            let name = if thumb { suffix.to_string() } else { format!("{}{suffix}", d.name()) };
            let b = &h.bones[bone(h, &name)];
            let seg = if thumb {
                if name == "thumb_mcp" { Seg::Proximal } else { Seg::Distal }
            } else {
                match *suffix {
                    "_mcp" => Seg::Proximal,
                    "_pip" => Seg::Middle,
                    _ => Seg::Distal,
                }
            };
            let r = h.capsules.iter().find(|c| c.digit == Some(d) && c.seg == seg).unwrap().r;
            let y = at * b.len;
            let pts = [dvec2(-0.62 * r, y + 0.0003), dvec2(0.0, y), dvec2(0.62 * r, y + 0.0003)];
            cuts.extend(crease(field, b.frame, &pts, 0.003, *depth, 0.001));
        }
    }
    cuts
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
