//! Hands that do not pass through each other while they move.
//!
//! A clip's hands (any number, of any people) are run once, at parse time, through a small
//! simulation of where each hand is pushed to, on a fixed time grid: each hand follows its own
//! keyed motion plus a push (a translation of the whole hand and forearm); every step, wherever
//! two hands' capsules overlap (or come within `SKIN` of touching), the hands are moved apart
//! along the contact normals, and a critically damped spring pulls each push back to zero.
//! Because the pushed hands never sink in, the normals come from shallow contacts and cannot
//! flip from one side of a hand to the other: a hand crossing another slides over it and settles
//! back onto its own path.
//!
//! Which side a hand passes on follows the keyed motion: for each stretch of time where the
//! keyed hands overlap, the direction that separates them with the least movement at its
//! deepest overlap (a hand keyed lying behind another goes behind it) tilts the contact normals,
//! so an edge-on first touch does not send the hand over when it was keyed under.
//!
//! The push goes to the hand that moves faster (by wrist speed at that moment, blended smoothly;
//! at equal speeds, the one listed later), corrections are limited in speed and fade in over
//! `RAMP` seconds when collisions turn on, so nothing pops. `Clip::sample(t)` reads the push at
//! `t` from the table: still a pure function of time.

use glam::DVec3;

use crate::query::seg_closest;
use crate::rig::HandGeom;

/// Steps per second.
pub const RATE: f64 = 240.0;
/// Hands are kept this far apart (m), so a contact reads as touching, not overlapping.
pub const SKIN: f64 = 0.001;
/// The spring that pulls a push back to zero: natural frequency (Hz), critically damped.
const RETURN_HZ: f64 = 2.5;
/// The fastest a push corrects (m/s), so a sudden overlap (collisions turned on mid-overlap)
/// resolves over a few frames instead of in one.
const MAX_SPEED: f64 = 5.0;
/// Seconds over which collisions between two hands come on when a key turns them on (an
/// overlap left from before then opens gradually, by `EASE` seconds' time constant) and fade
/// out when one turns them off.
pub const RAMP: f64 = 0.4;
const RAMP_OUT: f64 = 0.2;
const EASE: f64 = 0.12;
const ITERATIONS: usize = 4;
/// How strongly an overlap's side tilts the contact normals (1 = 45 degrees when edge-on).
const SIDE_BIAS: f64 = 0.8;
/// Seconds before and after an overlap of the keyed hands that its side applies.
const SIDE_LEAD: f64 = 0.3;

/// Two hands' parts that may overlap (a held goal joins them): capsule masks of hand `a` and
/// hand `b` (indices into the moment's hands), or `None` for the whole hands.
#[derive(Clone)]
pub struct Exempt {
    pub a: usize,
    pub b: usize,
    pub parts: Option<(Vec<bool>, Vec<bool>)>,
}

/// The hands at one time, as their own motion has them.
pub struct Moment {
    pub hands: Vec<HandGeom>,
    pub wrists: Vec<DVec3>,
    /// Per hand: whether it collides at all now.
    pub on: Vec<bool>,
    pub exempt: Vec<Exempt>,
}

/// A hand's capsules (a, b, radius) and their bounding box.
struct Body {
    caps: Vec<(DVec3, DVec3, f64)>,
    lo: DVec3,
    hi: DVec3,
}

impl Body {
    fn of(h: &HandGeom) -> Body {
        let (lo, hi) = bounds(h);
        Body { caps: h.capsules.iter().map(|c| (c.a, c.b, c.r)).collect(), lo, hi }
    }

    fn near(&self, pa: DVec3, other: &Body, pb: DVec3, margin: f64) -> bool {
        boxes_near((self.lo + pa, self.hi + pa), (other.lo + pb, other.hi + pb), margin)
    }
}

struct Step {
    bodies: Vec<Body>,
    wrists: Vec<DVec3>,
    on: Vec<bool>,
    exempt: Vec<Exempt>,
}

impl Step {
    /// The capsule masks of hands `a` and `b` (in that order) that may overlap.
    fn masks(&self, a: usize, b: usize) -> Vec<(&[bool], &[bool])> {
        self.exempt
            .iter()
            .filter_map(|e| match &e.parts {
                Some((x, y)) if (e.a, e.b) == (a, b) => Some((x.as_slice(), y.as_slice())),
                Some((x, y)) if (e.b, e.a) == (a, b) => Some((y.as_slice(), x.as_slice())),
                _ => None,
            })
            .collect()
    }

    fn whole_exempt(&self, a: usize, b: usize) -> bool {
        self.exempt.iter().any(|e| e.parts.is_none() && ((e.a, e.b) == (a, b) || (e.b, e.a) == (a, b)))
    }
}

/// Each hand's push over time.
pub struct Track {
    start: f64,
    /// Per step, per hand.
    push: Vec<Vec<DVec3>>,
}

impl Track {
    pub fn none() -> Track {
        Track { start: 0.0, push: Vec::new() }
    }

    /// Hand `hand`'s push at `t` (linear between steps; the first or last step outside them).
    pub fn at(&self, hand: usize, t: f64) -> DVec3 {
        if self.push.is_empty() {
            return DVec3::ZERO;
        }
        let x = ((t - self.start) * RATE).clamp(0.0, (self.push.len() - 1) as f64);
        let k = (x.floor() as usize).min(self.push.len().saturating_sub(2));
        let f = x - k as f64;
        let a = self.push[k][hand];
        let b = self.push.get(k + 1).map_or(a, |p| p[hand]);
        a + (b - a) * f
    }
}

/// How much of a pair's push goes to the later hand, from the hands' speeds: all of it unless
/// the earlier hand moves clearly faster (about twice as fast), blended in between.
fn later_share(earlier: f64, later: f64) -> f64 {
    let x = (later - earlier) / (later + earlier + 0.05);
    let u = ((x + 0.3) / 0.3).clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

/// The bounding box of a hand's capsules.
pub fn bounds(h: &HandGeom) -> (DVec3, DVec3) {
    h.capsules.iter().fold((DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY)), |(lo, hi), c| {
        (lo.min(c.a.min(c.b) - DVec3::splat(c.r)), hi.max(c.a.max(c.b) + DVec3::splat(c.r)))
    })
}

/// Whether two boxes come within `margin` of each other.
pub fn boxes_near(a: (DVec3, DVec3), b: (DVec3, DVec3), margin: f64) -> bool {
    let m = DVec3::splat(margin);
    !((a.0 - m).cmpgt(b.1).any() || (b.0 - m).cmpgt(a.1).any())
}

/// How deep two hands (moved by `pa`, `pb`) overlap: the largest amount any capsule pair is
/// closer than `SKIN` apart (0 when none is).
fn depth(a: &Body, pa: DVec3, b: &Body, pb: DVec3, exempt: &[(&[bool], &[bool])]) -> f64 {
    if !a.near(pa, b, pb, SKIN) {
        return 0.0;
    }
    let mut deepest = 0.0_f64;
    for (i, ca) in a.caps.iter().enumerate() {
        for (j, cb) in b.caps.iter().enumerate() {
            if exempt.iter().any(|(ma, mb)| ma[i] && mb[j]) {
                continue;
            }
            let (x, y) = seg_closest(ca.0 + pa, ca.1 + pa, cb.0 + pb, cb.1 + pb);
            deepest = deepest.max(ca.2 + cb.2 + SKIN - x.distance(y));
        }
    }
    deepest
}

/// Moves two hands (by their pushes `pa`, `pb`) apart wherever their capsules overlap or come
/// within `SKIN`, one capsule pair at a time (each sees the moves before it); `share` of each
/// correction goes to `b`, `strength` scales it and the corrections stop when they add up to
/// `budget` (meters). `side` (from b toward a, or zero) tilts the normals toward the side the
/// keyed motion passes on.
#[allow(clippy::too_many_arguments)]
fn separate(a: &Body, pa: &mut DVec3, b: &Body, pb: &mut DVec3, exempt: &[(&[bool], &[bool])], share: f64, strength: f64, side: DVec3, budget: &mut f64) {
    for (i, ca) in a.caps.iter().enumerate() {
        for (j, cb) in b.caps.iter().enumerate() {
            if *budget <= 0.0 {
                return;
            }
            if exempt.iter().any(|(ma, mb)| ma[i] && mb[j]) {
                continue;
            }
            let (x, y) = seg_closest(ca.0 + *pa, ca.1 + *pa, cb.0 + *pb, cb.1 + *pb);
            let d = x.distance(y);
            let need = ca.2 + cb.2 + SKIN - d;
            if need <= 0.0 {
                continue;
            }
            let n = if d > 1e-7 { (x - y) / d } else { (ca.0 + ca.1 - cb.0 - cb.1).normalize_or(DVec3::Y) };
            // |n + 0.8 side| >= 0.2, so the tilted normal is never zero and turns continuously.
            let dir = (n + side * SIDE_BIAS).normalize();
            let c = (need * strength / dir.dot(n).max(0.3)).min(*budget);
            *budget -= c;
            *pa += dir * c * (1.0 - share);
            *pb -= dir * c * share;
        }
    }
}

/// Unit directions spread evenly over the sphere.
fn directions(n: usize) -> Vec<DVec3> {
    let golden = std::f64::consts::PI * (3.0 - 5f64.sqrt());
    (0..n)
        .map(|i| {
            let y = 1.0 - 2.0 * (i as f64 + 0.5) / n as f64;
            let r = (1.0 - y * y).sqrt();
            let phi = golden * i as f64;
            DVec3::new(r * phi.cos(), y, r * phi.sin())
        })
        .collect()
}

/// The direction to move hand `a` (and `b` the other way) that separates them with the least
/// movement, among evenly spread directions.
fn least_separation(a: &Body, b: &Body, exempt: &[(&[bool], &[bool])]) -> DVec3 {
    const REACH: f64 = 0.3;
    let mut best = (f64::INFINITY, DVec3::ZERO);
    for u in directions(64) {
        // Per capsule pair, the interval of moves s along u at which they overlap (the gap is
        // convex in s, so it is one interval).
        let mut spans: Vec<(f64, f64)> = Vec::new();
        for (i, ca) in a.caps.iter().enumerate() {
            for (j, cb) in b.caps.iter().enumerate() {
                if exempt.iter().any(|(ma, mb)| ma[i] && mb[j]) {
                    continue;
                }
                let r = ca.2 + cb.2 + SKIN;
                let reach = (ca.1 - ca.0).length() / 2.0 + (cb.1 - cb.0).length() / 2.0 + r + REACH;
                if ((ca.0 + ca.1) / 2.0).distance((cb.0 + cb.1) / 2.0) > reach {
                    continue;
                }
                let gap = |s: f64| {
                    let (x, y) = seg_closest(ca.0 + u * s, ca.1 + u * s, cb.0, cb.1);
                    x.distance(y) - r
                };
                // Golden-section search for the closest approach, then the two crossings.
                let (mut lo, mut hi) = (-REACH, REACH);
                let k = 0.618_033_988_75;
                let (mut x1, mut x2) = (hi - k * (hi - lo), lo + k * (hi - lo));
                let (mut f1, mut f2) = (gap(x1), gap(x2));
                for _ in 0..24 {
                    if f1 < f2 {
                        hi = x2;
                        (x2, f2) = (x1, f1);
                        x1 = hi - k * (hi - lo);
                        f1 = gap(x1);
                    } else {
                        lo = x1;
                        (x1, f1) = (x2, f2);
                        x2 = lo + k * (hi - lo);
                        f2 = gap(x2);
                    }
                }
                let m = (lo + hi) / 2.0;
                if gap(m) >= 0.0 {
                    continue;
                }
                let crossing = |mut inside: f64, mut outside: f64| {
                    if gap(outside) < 0.0 {
                        return outside;
                    }
                    for _ in 0..20 {
                        let mid = (inside + outside) / 2.0;
                        if gap(mid) < 0.0 { inside = mid } else { outside = mid }
                    }
                    outside
                };
                spans.push((crossing(m, -REACH), crossing(m, REACH)));
            }
        }
        // The least move s >= 0 outside every span.
        spans.sort_by(|x, y| x.0.total_cmp(&y.0));
        let mut s = 0.0_f64;
        for (s1, s2) in &spans {
            if *s1 < s && *s2 > s {
                s = *s2;
            }
        }
        if s < best.0 {
            best = (s, u);
        }
    }
    best.1
}

/// Per pair of hands (a < b): the stretches of steps where the keyed hands overlap, each with
/// the side `a` passes `b` on.
fn sides(steps: &[Step]) -> Vec<Vec<Vec<(usize, usize, DVec3)>>> {
    let n = steps.first().map_or(0, |s| s.bodies.len());
    let mut out = vec![vec![Vec::new(); n]; n];
    for a in 0..n {
        for b in a + 1..n {
            let depths: Vec<f64> = steps.iter().map(|s| depth(&s.bodies[a], DVec3::ZERO, &s.bodies[b], DVec3::ZERO, &s.masks(a, b))).collect();
            let mut k = 0;
            while k < depths.len() {
                if depths[k] <= 0.0 {
                    k += 1;
                    continue;
                }
                let k0 = k;
                while k < depths.len() && depths[k] > 0.0 {
                    k += 1;
                }
                let deepest = (k0..k).max_by(|x, y| depths[*x].total_cmp(&depths[*y])).unwrap();
                let s = &steps[deepest];
                out[a][b].push((k0, k - 1, least_separation(&s.bodies[a], &s.bodies[b], &s.masks(a, b))));
            }
        }
    }
    out
}

/// Runs the hands from `start` to `end`; `moment(t)` gives the hands' own motion at `t`.
pub fn simulate(start: f64, end: f64, moment: impl Fn(f64) -> Result<Moment, String>) -> Result<Track, String> {
    let h = 1.0 / RATE;
    let count = ((end - start) * RATE).ceil().max(1.0) as usize + 1;
    let steps: Vec<Step> = (0..count)
        .map(|k| {
            let m = moment(start + k as f64 * h)?;
            Ok(Step { bodies: m.hands.iter().map(Body::of).collect(), wrists: m.wrists, on: m.on, exempt: m.exempt })
        })
        .collect::<Result<_, String>>()?;
    let sides = sides(&steps);
    let lead = (SIDE_LEAD * RATE) as usize;
    let n = steps[0].bodies.len();
    let w = std::f64::consts::TAU * RETURN_HZ;
    let mut push: Vec<Vec<DVec3>> = Vec::with_capacity(count);
    let mut p = vec![DVec3::ZERO; n];
    let mut v = vec![DVec3::ZERO; n];
    // Per pair (a < b): how far collisions are on, 0 to 1.
    let mut on = vec![vec![0.0; n]; n];
    for (k, m) in steps.iter().enumerate() {
        let prev = &steps[k.saturating_sub(1)].wrists;
        let speed: Vec<f64> = m.wrists.iter().zip(prev).map(|(a, b)| a.distance(*b) / h).collect();
        // The spring back toward each hand's own path.
        let mut q: Vec<DVec3> = (0..n)
            .map(|i| {
                let acc = -w * w * p[i] - 2.0 * w * v[i];
                v[i] += acc * h;
                p[i] + v[i] * h
            })
            .collect();
        let pred = q.clone();
        let on_before = on.clone();
        for a in 0..n {
            for b in a + 1..n {
                let want = if m.on[a] && m.on[b] && !m.whole_exempt(a, b) { 1.0 } else { 0.0 };
                on[a][b] = if k == 0 { want } else if want > on[a][b] { (on[a][b] + h / RAMP).min(want) } else { (on[a][b] - h / RAMP_OUT).max(want) };
            }
        }
        // How far each pair may still be moved apart this step: a pair coming on opens an old
        // overlap gradually (a part of its depth per step), a pair that is on keeps up with any
        // move up to MAX_SPEED.
        let mut budget = vec![vec![MAX_SPEED * h; n]; n];
        for a in 0..n {
            for b in a + 1..n {
                if k == 0 {
                    budget[a][b] = f64::INFINITY;
                } else if on[a][b] < 1.0 && on[a][b] > on_before[a][b] {
                    budget[a][b] = depth(&m.bodies[a], q[a], &m.bodies[b], q[b], &m.masks(a, b)) * h / EASE;
                }
            }
        }
        // The first step starts the clip resolved; later steps correct a little at a time.
        let iterations = if k == 0 { 40 } else { ITERATIONS };
        for _ in 0..iterations {
            for a in 0..n {
                for b in a + 1..n {
                    // Fading out, the corrections weaken; coming on, the budget limits them.
                    let strength = if on[a][b] < on_before[a][b] { on[a][b] } else if on[a][b] > 0.0 { 1.0 } else { 0.0 };
                    if strength <= 0.0 || !m.bodies[a].near(q[a], &m.bodies[b], q[b], SKIN) {
                        continue;
                    }
                    let share = later_share(speed[a], speed[b]);
                    // The side of the keyed overlap this step falls in (or leads into, or
                    // follows), the nearest one.
                    let side = sides[a][b]
                        .iter()
                        .filter(|(k0, k1, _)| k + lead >= *k0 && k <= k1 + lead)
                        .min_by_key(|(k0, k1, _)| if k < *k0 { k0 - k } else { k.saturating_sub(*k1) })
                        .map_or(DVec3::ZERO, |s| s.2);
                    let (mut qa, mut qb) = (q[a], q[b]);
                    separate(&m.bodies[a], &mut qa, &m.bodies[b], &mut qb, &m.masks(a, b), share, strength, side, &mut budget[a][b]);
                    (q[a], q[b]) = (qa, qb);
                }
            }
        }
        for i in 0..n {
            let mut dq = q[i] - pred[i];
            if k > 0 && dq.length() > MAX_SPEED * h {
                dq *= MAX_SPEED * h / dq.length();
            }
            p[i] = pred[i] + dq;
            // An inelastic contact: the correction stops the push's motion into the other
            // hand but adds none away from it (no bounce, and no pressure built up against it).
            let len = dq.length();
            if len > 1e-12 {
                let d = dq / len;
                let vn = v[i].dot(d);
                if vn < 0.0 {
                    v[i] += d * ((vn + len / h).min(0.0) - vn);
                }
            }
        }
        push.push(p.clone());
    }
    Ok(Track { start, push })
}
