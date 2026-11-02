//! Motion: a clip is a list of keyed poses, and `Clip::sample(t)` is a pure function of time
//! (fframes renders frames out of order on many threads; HyperFrames and Remotion seek).
//!
//! Every joint value is the first key's value plus one smoothed step per later key. A step is a
//! minimum-jerk drive (the bell-shaped speed of human reaching) filtered by a damped spring,
//! evaluated in closed form, so moves ease in and out, can overshoot and settle, and later keys
//! can start before earlier ones finish. On top of that: a finger cascade, slower springs for
//! distal joints, fingers that lag behind fast hand moves, and a seeded idle layer. Goals that
//! two neighbouring keys share are re-solved every frame, so a held contact stays closed while
//! the hand moves.

use glam::{DAffine3, DQuat, DVec3, EulerRot};
use serde_json::{Value, json};

use crate::collide;
use crate::params::PARAMS;
use crate::pose::{self, HandId, Side};
use crate::rig::{Digit, FOREARM, HandGeom, Seg};

const DIGITS: [&str; 5] = ["thumb", "index", "middle", "ring", "pinky"];
/// Key fields that are about timing (and collisions), not the pose. All but `t` can also go on
/// a hand.
const TIMING: [&str; 8] = ["t", "move", "ease", "spring", "cascade", "anticipate", "pass", "collide"];

#[derive(Clone, Copy, Debug)]
struct Spring {
    /// Natural frequency, rad/s; infinite = no spring (the drive alone).
    w: f64,
    zeta: f64,
}

impl Spring {
    fn scaled(self, k: f64) -> Spring {
        Spring { w: self.w * k, zeta: self.zeta }
    }

    /// How long the spring trails its input, about.
    fn lag(self) -> f64 {
        if self.w.is_finite() { 2.0 * self.zeta / self.w } else { 0.0 }
    }

    /// Unit step response at `tau` seconds.
    fn step(self, tau: f64) -> f64 {
        if tau <= 0.0 {
            return 0.0;
        }
        let (w, z) = (self.w, self.zeta);
        if z >= 0.999 {
            1.0 - (1.0 + w * tau) * (-w * tau).exp()
        } else {
            let wd = w * (1.0 - z * z).sqrt();
            1.0 - (-z * w * tau).exp() * ((wd * tau).cos() + z * w / wd * (wd * tau).sin())
        }
    }

    /// When the response has settled to well under 1%.
    fn settled(self) -> f64 {
        if self.w.is_finite() { 7.0 / (self.zeta.min(1.0) * self.w) } else { 0.0 }
    }
}

#[derive(Clone, Copy, Debug)]
enum Shape {
    MinJerk,
    Linear,
}

/// One step from 0 to 1: an optional anticipation (a small move the other way first), then the
/// drive, then the spring.
#[derive(Clone, Copy, Debug)]
struct Step {
    start: f64,
    dur: f64,
    anticipate: f64,
    ant_dur: f64,
    shape: Shape,
    spring: Spring,
}

fn min_jerk(u: f64) -> f64 {
    let u = u.clamp(0.0, 1.0);
    u * u * u * (10.0 - 15.0 * u + 6.0 * u * u)
}

fn min_jerk_rate(u: f64) -> f64 {
    if !(0.0..=1.0).contains(&u) {
        return 0.0;
    }
    30.0 * u * u * (1.0 - u) * (1.0 - u)
}

// 16-point Gauss-Legendre nodes and weights on [-1, 1] (positive half).
const GL_X: [f64; 8] = [0.0950125098376374, 0.2816035507792589, 0.4580167776572274, 0.6178762444026438, 0.7554044083550030, 0.8656312023878318, 0.9445750230732326, 0.9894009349916499];
const GL_W: [f64; 8] = [0.1894506104550685, 0.1826034150449236, 0.1691565193950025, 0.1495959888165767, 0.1246289712555339, 0.0951585116824928, 0.0622535239386479, 0.0271524594117541];

fn integrate(a: f64, b: f64, f: impl Fn(f64) -> f64) -> f64 {
    if b <= a {
        return 0.0;
    }
    let (m, h) = ((a + b) / 2.0, (b - a) / 2.0);
    let mut s = 0.0;
    for (x, w) in GL_X.iter().zip(GL_W) {
        s += w * (f(m - h * x) + f(m + h * x));
    }
    s * h
}

impl Step {
    fn begin(&self) -> f64 {
        self.start - if self.anticipate != 0.0 { self.ant_dur } else { 0.0 }
    }

    fn end(&self) -> f64 {
        self.start + self.dur
    }

    /// The drive's pieces: (from, to, value before, value after).
    fn pieces(&self) -> [(f64, f64, f64, f64); 2] {
        let a = self.anticipate;
        [(self.start - self.ant_dur, self.start, 0.0, -a), (self.start, self.start + self.dur, -a, 1.0)]
    }

    fn rate(&self, from: f64, to: f64, s: f64) -> f64 {
        let d = to - from;
        match self.shape {
            Shape::MinJerk => min_jerk_rate((s - from) / d) / d,
            Shape::Linear => {
                if (from..=to).contains(&s) { 1.0 / d } else { 0.0 }
            }
        }
    }

    fn drive(&self, t: f64) -> f64 {
        let mut x = 0.0;
        for (from, to, v0, v1) in self.pieces() {
            if v0 == v1 || t <= from {
                continue;
            }
            let u = if to - from < 1e-9 { 1.0 } else { (t - from) / (to - from) };
            let f = match self.shape {
                Shape::MinJerk => min_jerk(u),
                Shape::Linear => u.clamp(0.0, 1.0),
            };
            x = v0 + (v1 - v0) * f;
        }
        x
    }

    /// The step's value at `t`: 0 before it begins, 1 once settled.
    fn at(&self, t: f64) -> f64 {
        if t <= self.begin() {
            return 0.0;
        }
        if !self.spring.w.is_finite() {
            return self.drive(t);
        }
        if t > self.end() + self.spring.settled() {
            return 1.0;
        }
        // The spring's response to the drive: the drive's rate convolved with the spring's
        // step response.
        let mut x = 0.0;
        for (from, to, v0, v1) in self.pieces() {
            if v0 == v1 || t <= from {
                continue;
            }
            if to - from < 1e-9 {
                x += (v1 - v0) * self.spring.step(t - from);
                continue;
            }
            let hi = t.min(to);
            x += (v1 - v0) * integrate(from, hi, |s| self.rate(from, to, s) * self.spring.step(t - s));
        }
        x
    }
}

/// A hand at one key: every joint parameter (in `PARAMS` order) and the placement.
#[derive(Clone, Debug)]
struct HandKey {
    values: Vec<f64>,
    /// The wrist position and the forearm's orientation (the hand turns about the wrist).
    wrist: DVec3,
    rot: DQuat,
}

/// Per hand, in the clip's hand order (`Clip::hands`).
#[derive(Clone, Debug)]
struct Key {
    t: f64,
    hands: Vec<Option<HandKey>>,
    goals: Vec<Value>,
    camera: Option<Value>,
    /// The step into this key, per joint group (see `group_of`), per hand.
    steps: Vec<Vec<Step>>,
    /// The step for each hand's placement.
    place_steps: Vec<Step>,
    /// The step for the camera.
    cam_step: Step,
    /// Per hand: the hand moves through this key without stopping.
    pass: Vec<bool>,
    cam_pass: bool,
    /// The hands this key writes (a hand it leaves out moves on between its own keys).
    named: Vec<bool>,
    /// Per hand: whether it collides with other hands during the move into this key and while
    /// it holds it.
    collide: Vec<bool>,
    /// Per hand: its hand frame at this key (`grip::hand_frame`).
    frames: Vec<DAffine3>,
}

/// A grip held by a run of keys: the follower moves with the leader.
struct Hold {
    /// The grip's identity (`grip_id`).
    id: (Value, Value),
    leader: usize,
    follower: usize,
    first: usize,
    last: usize,
    /// Per key of the run: the follower's hand frame in the leader's.
    offsets: Vec<DAffine3>,
    /// The follower is joined to the leader from `join.1` on (fading in from `join.0`) and lets
    /// go from `part.0` (gone at `part.1`); infinite when the run starts or ends the clip.
    join: (f64, f64),
    part: (f64, f64),
    /// When the gripping fingers start to open (the grip is held until then).
    release: f64,
    /// While the pair does not collide.
    exempt: (f64, f64),
    /// Per hand (leader, follower): the joint values (PARAMS indices) of its gripping digits.
    closing: [Vec<usize>; 2],
}

/// How fast a gripped follower's forearm follows its hand (Hz, critically damped): the elbow
/// trails a pull by about 2 / (2 pi TRAIL_HZ) s, a few frames.
const TRAIL_HZ: f64 = 3.0;
/// Seconds over which gripping fingers close before a grip's key, and open as it ends; the
/// hands start apart this long after the fingers start opening.
const GRIP_CLOSE: f64 = 0.3;
const GRIP_OPEN: f64 = 0.25;
const GRIP_PART_DELAY: f64 = 0.2;

impl Hold {
    /// How joined the follower is at `t`, 0 to 1.
    fn weight(&self, t: f64) -> f64 {
        let ramp = |t: f64, (a, b): (f64, f64)| if t >= b { 1.0 } else if t <= a { 0.0 } else { smooth((t - a) / (b - a)) };
        ramp(t, self.join) * (1.0 - ramp(t, self.part))
    }
}

/// Between two hand frames: positions straight, orientations along the shortest turn.
fn blend(a: DAffine3, b: DAffine3, w: f64) -> DAffine3 {
    if w <= 0.0 {
        return a;
    }
    if w >= 1.0 {
        return b;
    }
    let (qa, qb) = (DQuat::from_mat3(&a.matrix3.into()), DQuat::from_mat3(&b.matrix3.into()));
    let qb = if qa.dot(qb) < 0.0 { -qb } else { qb };
    DAffine3::from_rotation_translation(qa.slerp(qb, w), a.translation.lerp(b.translation, w))
}

/// The wrist's rotation (twist, deviation, flexion) of a hand, as the rig builds it.
fn wrist_rot(side: Side, values: &[f64]) -> DQuat {
    let idx = |k: &str| PARAMS.iter().position(|p| p.key == k).unwrap();
    let (twist, dev, flex) = (values[idx("twist")], values[idx("wrist_dev")], values[idx("wrist_flex")]);
    let local = DQuat::from_rotation_y(twist.to_radians()) * DQuat::from_rotation_z(-dev.to_radians()) * DQuat::from_rotation_x(flex.to_radians());
    // The left hand is built mirrored in x: M R M keeps x rotations and flips y and z.
    if side == Side::Left { DQuat::from_xyzw(local.x, -local.y, -local.z, local.w) } else { local }
}

/// A hand's frame from its forearm placement and joint values.
fn hand_frame(side: Side, values: &[f64], wrist: DVec3, rot: DQuat) -> DAffine3 {
    DAffine3::from_rotation_translation(rot * wrist_rot(side, values), wrist)
}

/// The direction from the wrist toward the elbow, in the hand's frame, for these wrist angles.
fn forearm_dir(side: Side, dev: f64, flex: f64) -> DVec3 {
    let (d, f) = (dev.to_radians(), flex.to_radians());
    let x = if side == Side::Left { -d.sin() } else { d.sin() };
    DVec3::new(x, -d.cos() * f.cos(), d.cos() * f.sin())
}

impl Key {
    fn hand_steps(&mut self, si: usize) -> impl Iterator<Item = &mut Step> {
        self.steps[si].iter_mut().chain(std::iter::once(&mut self.place_steps[si]))
    }

    /// When the move into this key begins (for goals).
    fn begin(&self) -> f64 {
        (0..self.named.len()).filter(|si| self.named[*si]).map(|si| self.place_steps[si].begin()).fold(self.cam_step.begin().min(self.t), f64::min)
    }
}

/// The timing of a move into a key, from its timing fields.
struct Timing {
    base: Step,
    spring: Spring,
    cascade: f64,
    pass: bool,
}

/// `f` holds the timing fields (the key's, with a hand's own over them); `gap` is the time since
/// the previous key, the default move.
fn timing(f: &Value, t: f64, gap: f64) -> Result<Timing, String> {
    let mv = num(f, "move")?.unwrap_or(gap).max(0.0);
    let (shape, mut spring) = match f.get("ease").and_then(Value::as_str).unwrap_or("natural") {
        "natural" => (Shape::MinJerk, Spring { w: std::f64::consts::TAU * 4.0, zeta: 0.7 }),
        "snappy" => (Shape::MinJerk, Spring { w: std::f64::consts::TAU * 6.0, zeta: 0.45 }),
        "smooth" => (Shape::MinJerk, Spring { w: f64::INFINITY, zeta: 1.0 }),
        "linear" => (Shape::Linear, Spring { w: f64::INFINITY, zeta: 1.0 }),
        e => return Err(format!("ease \"{e}\": use natural, snappy, smooth or linear")),
    };
    if let Some(s) = f.get("spring") {
        let hz = num(s, "hz")?.unwrap_or(spring.w / std::f64::consts::TAU);
        let zeta = num(s, "damping")?.unwrap_or(spring.zeta);
        if !(0.2..=1.0).contains(&zeta) || hz <= 0.0 {
            return Err("spring: hz > 0 and damping from 0.2 (bouncy) to 1 (no overshoot)".into());
        }
        spring = Spring { w: std::f64::consts::TAU * hz, zeta };
    }
    let anticipate = num(f, "anticipate")?.unwrap_or(0.0);
    if !(0.0..=0.5).contains(&anticipate) {
        return Err("anticipate is a fraction of the move, 0 to 0.5 (0.1 is a lot)".into());
    }
    let pass = match f.get("pass") {
        None => false,
        Some(Value::Bool(b)) => *b,
        Some(_) => return Err("pass is true or false".into()),
    };
    let cascade = num(f, "cascade")?.unwrap_or((mv * 0.12).min(0.035));
    // `t` is when the hand arrives: the drive ends a spring's lag earlier.
    let start = t - spring.lag() - mv;
    let base = Step { start, dur: mv, anticipate, ant_dur: (mv * 0.4).clamp(0.08, 0.3), shape, spring };
    Ok(Timing { base, spring, cascade, pass })
}

/// Joint groups for timing: the cascade shifts each digit, distal joints get slower springs.
fn group_of(key: &str) -> (Option<usize>, f64) {
    let (digit, joint) = key.split_once('.').unwrap_or(("", key));
    let d = DIGITS.iter().position(|x| *x == digit);
    let k = match joint {
        "pip_flex" | "mcp_flex" if digit == "thumb" => 0.85,
        "pip_flex" => 0.85,
        "dip_flex" | "ip_flex" => 0.72,
        "palm_cup" => 0.9,
        _ => 1.0,
    };
    (d, k)
}

pub struct Clip {
    pub duration: f64,
    pub fps: f64,
    pub camera: Option<Value>,
    /// The video's [width, height], when the clip gives it.
    pub size: Option<Value>,
    /// How to draw the hands (`look::Look`), copied into every sampled pose.
    pub look: Option<Value>,
    /// Per person: { "ana": { "look": {..} } }, copied into every sampled pose.
    pub people: Option<Value>,
    keys: Vec<Key>,
    idle: f64,
    seed: u64,
    follow: f64,
    /// The hands in the clip (all in the first key): the default person's right and left, then
    /// other people's hands in the order the first key lists them.
    ids: Vec<HandId>,
    /// Each hand's channel of the idle noise (the default hands keep the ones they always had).
    chan: Vec<usize>,
    /// How far collisions with the other hands push each hand, over time.
    track: collide::Track,
    /// Grips held over runs of keys.
    holds: Vec<Hold>,
}

/// A grip goal's identity: its two hands and style.
fn grip_id(g: &Value) -> Option<(Value, Value)> {
    Some((g.get("grip")?.clone(), g.get("style").cloned().unwrap_or(Value::Null)))
}

/// A sampled frame: the pose and the goals held at that time.
pub struct Frame {
    pub pose: Value,
    pub goals: Vec<Value>,
}

/// Where a site is in a frame: in the world, and through the clip's camera on screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SiteOnScreen {
    pub world: DVec3,
    /// Fractions of the frame, x from the left and y from the top (as `place.screen`); none
    /// without a camera, or behind it.
    pub screen: Option<[f64; 2]>,
    /// Meters from the camera along its view; none without a camera.
    pub depth: Option<f64>,
}

impl SiteOnScreen {
    /// `{"screen": [x, y], "depth": d, "world": [x, y, z]}`, null where there is none.
    pub fn json(&self) -> Value {
        let r = |x: f64| (x * 1e5).round() / 1e5 + 0.0;
        json!({
            "screen": self.screen.map(|s| [r(s[0]), r(s[1])]),
            "depth": self.depth.map(r),
            "world": [r(self.world.x), r(self.world.y), r(self.world.z)],
        })
    }
}

impl Frame {
    /// Where sites (`right.index.tip`, `ana.left.palm_center`; a segment gives its middle) are
    /// in this frame, with the frame's camera and size.
    pub fn sites_on_screen(&self, names: &[&str]) -> Result<Vec<SiteOnScreen>, String> {
        let solved = crate::solve_value(&self.pose)?;
        let view = self.pose.get("camera").and_then(pose::View::of);
        let aspect = pose::aspect(&self.pose);
        names
            .iter()
            .map(|name| {
                let target = solved.target(name)?;
                let world = target.site.map_or((target.a + target.b) / 2.0, |s| s.1);
                let (screen, depth) = match &view {
                    Some(v) => {
                        let (s, d) = v.project(aspect, world);
                        (s, Some(d))
                    }
                    None => (None, None),
                };
                Ok(SiteOnScreen { world, screen, depth })
            })
            .collect()
    }
}

fn num(v: &Value, k: &str) -> Result<Option<f64>, String> {
    match v.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(x) => x.as_f64().map(Some).ok_or(format!("\"{k}\" must be a number")),
    }
}

fn get_path<'a>(v: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.').try_fold(v, |v, k| v.get(k))
}

fn set_path(v: &mut Value, path: &str, x: Value) {
    let mut cur = v;
    let parts: Vec<&str> = path.split('.').collect();
    for k in &parts[..parts.len() - 1] {
        if !cur.get(*k).is_some_and(Value::is_object) {
            cur[*k] = json!({});
        }
        cur = cur.get_mut(*k).unwrap();
    }
    cur[parts[parts.len() - 1]] = x;
}

fn r6(x: f64) -> f64 {
    (x * 1e6).round() / 1e6 + 0.0
}

impl Clip {
    pub fn parse(text: &str) -> Result<Clip, String> {
        let v: Value = serde_json::from_str(text).map_err(|e| format!("invalid JSON: {e}"))?;
        Clip::from_value(&v)
    }

    pub fn from_value(v: &Value) -> Result<Clip, String> {
        let obj = v.as_object().ok_or("a clip is a JSON object")?;
        for k in obj.keys() {
            if !["version", "name", "description", "duration", "fps", "camera", "idle", "follow", "keys", "size", "look", "people", "collide"].contains(&k.as_str()) {
                return Err(format!("unknown clip field \"{k}\" (fields: version, name, description, duration, fps, size, camera, look, people, idle, follow, collide, keys)"));
            }
        }
        if let Some(ver) = num(v, "version")? {
            if ver != 1.0 {
                return Err(format!("clip version {ver} is not supported (this build reads version 1)"));
            }
        }
        let fps = num(v, "fps")?.unwrap_or(30.0);
        let (idle, seed) = match v.get("idle") {
            None => (1.0, 0),
            Some(Value::Bool(b)) => (if *b { 1.0 } else { 0.0 }, 0),
            Some(Value::Number(n)) => (n.as_f64().unwrap(), 0),
            Some(o) => (num(o, "amount")?.unwrap_or(1.0), num(o, "seed")?.unwrap_or(0.0) as u64),
        };
        let follow = num(v, "follow")?.unwrap_or(1.0);
        let size = v.get("size").cloned();
        let look = v.get("look").cloned();
        if let Some(l) = &look {
            crate::look::Look::from_value(l)?;
        }
        let people = v.get("people").cloned();
        if let Some(p) = &people {
            pose::check_people(p)?;
        }
        let collide_default = match v.get("collide") {
            None => true,
            Some(Value::Bool(b)) => *b,
            Some(_) => return Err("\"collide\" is true (hands do not pass through each other; the default) or false".into()),
        };
        if let Some(s) = &size {
            let ok = s.as_array().is_some_and(|a| a.len() == 2 && a.iter().all(|x| x.as_f64().is_some_and(|x| x > 0.0)));
            if !ok {
                return Err("\"size\" is [width, height] in pixels".into());
            }
        }
        let raw_keys = v.get("keys").and_then(Value::as_array).ok_or("a clip needs \"keys\": a list of keyed poses")?;
        if raw_keys.is_empty() {
            return Err("\"keys\" is empty".into());
        }

        let mut keys: Vec<Key> = Vec::new();
        let mut prev: Option<Value> = None;
        let mut camera = v.get("camera").cloned();
        // The camera as of the key being read, for screen placements.
        let mut cam_now = camera.clone();
        let mut ids: Vec<HandId> = Vec::new();
        // When each hand's last own key was.
        let mut last_t: Vec<f64> = Vec::new();
        for (i, raw) in raw_keys.iter().enumerate() {
            let at = |e: String| format!("keys[{i}]: {e}");
            let t = num(raw, "t").map_err(at)?.ok_or_else(|| at("needs \"t\" (seconds)".into()))?;
            if let Some(p) = keys.last() {
                if t < p.t {
                    return Err(at(format!("t={t} comes before the previous key's t={}", p.t)));
                }
            }
            let mut pose_part = raw.clone();
            for k in TIMING {
                pose_part.as_object_mut().ok_or_else(|| at("a key is a JSON object".into()))?.remove(k);
            }
            // A hand's own timing fields, over the key's.
            let mut key_timing = json!({});
            for k in TIMING {
                if let Some(x) = raw.get(k) {
                    key_timing[k] = x.clone();
                }
            }
            let mut own_timing: Vec<(String, Value)> = Vec::new();
            if let Some(hs) = pose_part.get_mut("hands").and_then(Value::as_object_mut) {
                for (name, h) in hs.iter_mut() {
                    let Some(h) = h.as_object_mut() else { continue };
                    if h.contains_key("t") {
                        return Err(at(format!("hands.{name}: t belongs to the key; a hand can have its own move, ease, spring, cascade, anticipate, pass and collide")));
                    }
                    let mut tm = key_timing.clone();
                    for k in &TIMING[1..] {
                        if let Some(x) = h.remove(*k) {
                            tm[*k] = x;
                        }
                    }
                    own_timing.push((name.clone(), tm));
                }
            }
            let mut merged = merge_key(prev.as_ref(), pose_part).map_err(at)?;
            if merged.get("version").is_none() {
                merged["version"] = json!(1);
            }
            // Screen placements need the camera and the frame size.
            if i == 0 {
                cam_now = cam_now.or_else(|| merged.get("camera").cloned());
            } else if let (Some(c), Some(k)) = (cam_now.as_mut(), raw.get("camera")) {
                pose::merge(c, k.clone());
            }
            if let Some(c) = &cam_now {
                merged["camera"] = c.clone();
            }
            if let Some(s) = &size {
                merged["size"] = s.clone();
            }
            // Each person's hand size, so keys are solved at the size they play at.
            if let Some(p) = &people {
                merged["people"] = p.clone();
            }
            // A grip the previous key held too keeps its hold: the follower goes where the
            // previous key's grip puts it relative to the leader, fingers as they were. It is
            // not solved again: a hold that is not quite met (a child's hand in an adult's)
            // would otherwise be placed afresh in every key, a little differently each time.
            let mut kept = Vec::new();
            if let Some(p) = keys.last() {
                for g in merged.get("goals").and_then(Value::as_array).cloned().unwrap_or_default() {
                    let Some(id) = grip_id(&g) else { continue };
                    if !p.goals.iter().any(|q| grip_id(q).as_ref() == Some(&id)) {
                        continue;
                    }
                    let Ok(goal) = serde_json::from_value::<pose::Goal>(g.clone()) else { continue };
                    let Some((la, lb)) = crate::grip::pair(&goal) else { continue };
                    let (Some(a), Some(b)) = (ids.iter().position(|h| *h == la), ids.iter().position(|h| *h == lb)) else { continue };
                    let s = crate::solve_value(&merged).map_err(at)?;
                    let Some(l) = s.hands.iter().find(|h| h.id == la) else { continue };
                    let f = crate::grip::hand_frame(l) * p.frames[a].inverse() * p.frames[b];
                    let place = json!({ "wrist": [f.translation.x, f.translation.y, f.translation.z], "palm_facing": [f.matrix3.z_axis.x, f.matrix3.z_axis.y, f.matrix3.z_axis.z], "fingers_toward": [f.matrix3.y_axis.x, f.matrix3.y_axis.y, f.matrix3.y_axis.z] });
                    if let Some(h) = merged["hands"].get_mut(lb.name()).and_then(Value::as_object_mut) {
                        for k in ["elbow", "forearm_rotation_deg", "mirror"] {
                            h.remove(k);
                        }
                        h.insert("place".into(), place);
                    }
                    kept.push(g);
                }
            }
            let mut to_solve = merged.clone();
            if let Some(goals) = to_solve.get_mut("goals").and_then(Value::as_array_mut) {
                goals.retain(|g| !kept.contains(g));
            }
            let has_goals = to_solve.get("goals").and_then(Value::as_array).is_some_and(|g| !g.is_empty());
            let mut solved_raw = if has_goals { crate::solve::solve_goals(&to_solve).map_err(at)?.0 } else { to_solve };
            if !kept.is_empty() {
                solved_raw["goals"] = merged["goals"].clone();
            }
            let (resolved, _) = pose::resolve(solved_raw.clone()).map_err(at)?;
            let solved = crate::solve_value(&solved_raw).map_err(at)?;
            let version = solved.pose.version;
            if i == 0 {
                ids = solved.hands.iter().map(|h| h.id.clone()).collect();
                if ids.is_empty() {
                    return Err(at("the first key has no hands".into()));
                }
                last_t = vec![0.0; ids.len()];
            }
            let n = ids.len();
            let mut hands: Vec<Option<HandKey>> = vec![None; n];
            let mut frames = Vec::with_capacity(n);
            for (si, id) in ids.iter().enumerate() {
                let name = id.name();
                let Some(g) = solved.hands.iter().find(|h| h.id == *id) else {
                    return Err(at(format!("the {name} hand is missing (a hand in the first key stays in every key)")));
                };
                frames.push(crate::grip::hand_frame(g));
                let hand = &resolved["hands"][&name];
                let values = PARAMS
                    .iter()
                    .map(|p| match get_path(hand, p.key).and_then(Value::as_f64) {
                        Some(x) => x,
                        None if p.key.ends_with("dip_flex") && version.unwrap_or(0) >= 1 => {
                            get_path(hand, &p.key.replace("dip_flex", "pip_flex")).and_then(Value::as_f64).unwrap_or(0.0) * 2.0 / 3.0
                        }
                        None => 0.0,
                    })
                    .collect();
                let mut values: Vec<f64> = values;
                for (k, x) in [("wrist_flex", g.wrist_angles[0]), ("wrist_dev", g.wrist_angles[1])] {
                    values[PARAMS.iter().position(|p| p.key == k).unwrap()] = x;
                }
                let rot = DQuat::from_mat3(&g.world.matrix3.into());
                let wrist = g.world.transform_point3(DVec3::new(0.0, FOREARM * g.scale, 0.0));
                hands[si] = Some(HandKey { values, wrist, rot });
            }
            if let Some(g) = solved.hands.iter().find(|h| !ids.contains(&h.id)) {
                return Err(at(format!("the {} hand is not in the first key; every hand must start in keys[0]", g.name)));
            }
            // The next key starts from this one, with every placement written out.
            let mut next = resolved.clone();
            for (si, id) in ids.iter().enumerate() {
                if let (Some(h), Some(obj)) = (&hands[si], next["hands"].get_mut(id.name()).and_then(Value::as_object_mut)) {
                    for k in ["place", "elbow", "forearm_rotation_deg", "mirror"] {
                        obj.remove(k);
                    }
                    let len = solved.hands.iter().find(|g| g.id == *id).map_or(FOREARM, |g| FOREARM * g.scale);
                    obj.insert("place".into(), place_json(h.wrist, h.rot, len));
                    for k in ["wrist_flex", "wrist_dev"] {
                        let j = PARAMS.iter().position(|p| p.key == k).unwrap();
                        obj.insert(k.into(), json!(r6(h.values[j])));
                    }
                }
            }
            let goals = next.as_object_mut().unwrap().remove("goals").and_then(|g| g.as_array().cloned()).unwrap_or_default();
            for k in ["name", "description", "checks", "probes", "size"] {
                next.as_object_mut().unwrap().remove(k);
            }
            if i == 0 && camera.is_none() {
                camera = next.get("camera").cloned();
            }
            let key_camera = raw.get("camera").cloned();
            next.as_object_mut().unwrap().remove("camera");
            prev = Some(next);

            // Timing of the step into this key: the key's own, and each hand's (its own fields
            // over the key's; its default move runs from that hand's last own key).
            let gap = keys.last().map(|p| t - p.t).unwrap_or(0.0);
            let cam = timing(raw, t, gap).map_err(at)?;
            let mut named = vec![false; n];
            for (si, id) in ids.iter().enumerate() {
                let name = id.name();
                let in_goals = raw.get("goals").is_some_and(|g| {
                    let text = g.to_string();
                    text.contains(&format!("\"{name}.")) || text.contains(&format!("\"{name}\""))
                });
                named[si] = i == 0 || raw.get("from").is_some() || in_goals || raw.get("hands").and_then(|h| h.get(&name)).is_some();
            }
            let mut steps: Vec<Vec<Step>> = vec![Vec::new(); n];
            let mut place_steps = vec![cam.base; n];
            let mut pass = vec![false; n];
            let mut collide = vec![collide_default; n];
            for si in 0..n {
                let name = ids[si].name();
                let fields = own_timing.iter().find(|(k, _)| *k == name).map(|(_, tm)| tm).unwrap_or(&key_timing);
                collide[si] = match fields.get("collide") {
                    None => collide_default,
                    Some(Value::Bool(b)) => *b,
                    Some(_) => return Err(at(format!("hands.{name}: collide is true or false"))),
                };
                let tm = timing(fields, t, if named[si] { t - last_t[si] } else { gap }).map_err(|e| at(format!("hands.{name}: {e}")))?;
                if named[si] {
                    last_t[si] = t;
                }
                place_steps[si] = tm.base;
                place_steps[si].spring = tm.spring.scaled(0.8);
                pass[si] = tm.pass && named[si];
                let (Some(p), Some(b)) = (keys.last(), &hands[si]) else { continue };
                let Some(a) = &p.hands[si] else { continue };
                // Closing runs pinky to index and then the thumb (it wraps over the fingers);
                // opening frees the thumb first, then index to pinky.
                let mut shift = [0.0; 5];
                for (d, name) in DIGITS.iter().enumerate() {
                    let net: f64 = PARAMS
                        .iter()
                        .enumerate()
                        .filter(|(_, q)| q.key.starts_with(&format!("{name}.")) && q.key.ends_with("_flex"))
                        .map(|(j, _)| b.values[j] - a.values[j])
                        .sum();
                    if net.abs() > 5.0 {
                        let rank = match (d, net > 0.0) {
                            (0, true) => 5.0,
                            (0, false) => -2.0,
                            (_, true) => (4 - d) as f64,
                            (_, false) => (d - 1) as f64,
                        };
                        shift[d] = (rank - 1.5) * tm.cascade;
                    }
                }
                steps[si] = PARAMS
                    .iter()
                    .map(|q| {
                        let (digit, k) = group_of(q.key);
                        let mut s = tm.base;
                        s.spring = tm.spring.scaled(k);
                        if let Some(d) = digit {
                            s.start += shift[d];
                        }
                        s
                    })
                    .collect();
            }
            let mut cam_step = cam.base;
            cam_step.spring = cam.spring.scaled(0.8);
            keys.push(Key { t, hands, goals, camera: key_camera, steps, place_steps, cam_step, pass, cam_pass: cam.pass, named, collide, frames });
        }
        // A pass key is a waypoint: its move runs on past it while the hand's next key starts
        // early, so the hand flows through it (near the key's pose) without slowing to a stop.
        for si in 0..ids.len() {
            for i in 1..keys.len() {
                if !keys[i].pass[si] {
                    continue;
                }
                let Some(j) = (i + 1..keys.len()).find(|j| keys[*j].named[si]) else { continue };
                let e = 0.5 * keys[i].place_steps[si].dur.min(keys[j].place_steps[si].dur);
                for s in keys[i].hand_steps(si) {
                    s.dur += e;
                }
                for s in keys[j].hand_steps(si) {
                    s.start -= e;
                    s.dur += e;
                }
            }
        }
        for i in 1..keys.len().saturating_sub(1) {
            if keys[i].cam_pass {
                let e = 0.5 * keys[i].cam_step.dur.min(keys[i + 1].cam_step.dur);
                keys[i].cam_step.dur += e;
                keys[i + 1].cam_step.start -= e;
                keys[i + 1].cam_step.dur += e;
            }
        }
        let holds = holds_of(&mut keys, &ids)?;
        let last = keys.last().unwrap().t;
        let duration = num(v, "duration")?.unwrap_or(last + 0.5);
        // The default hands keep the idle noise they always had; other hands get their own.
        let chan = ids
            .iter()
            .scan(2, |next, id| {
                Some(match (id.is_default(), id.side) {
                    (true, Side::Right) => 0,
                    (true, Side::Left) => 1,
                    _ => {
                        *next += 1;
                        *next - 1
                    }
                })
            })
            .collect();
        let mut clip = Clip { duration, fps, camera, size, look, people, keys, idle, seed, follow, ids, chan, track: collide::Track::none(), holds };
        if clip.ids.len() > 1 && clip.keys.iter().any(|k| k.collide.iter().filter(|c| **c).count() > 1) {
            clip.track = clip.collisions()?;
        }
        Ok(clip)
    }

    /// The clip's hands, in the order of `sample`'s pose and `track`.
    pub fn hands(&self) -> &[HandId] {
        &self.ids
    }

    pub fn frames(&self) -> usize {
        (self.duration * self.fps).round().max(1.0) as usize
    }

    /// The frame's width / height (from "size", default 16:9).
    pub fn aspect(&self) -> f64 {
        pose::aspect(&json!({ "size": self.size }))
    }

    /// Where a site is at `t` seconds: its world position, and its point on screen and depth
    /// through the clip's camera, e.g. to make a UI behind the hand react to a fingertip. For
    /// several sites at once, `sample(t)?.sites_on_screen(..)`.
    pub fn site_on_screen(&self, t: f64, name: &str) -> Result<SiteOnScreen, String> {
        Ok(self.sample(t)?.sites_on_screen(&[name])?[0])
    }

    /// The index of the last key at or before `t` (0 before the first).
    pub fn key_before(&self, t: f64) -> usize {
        self.keys.iter().rposition(|k| k.t <= t + 1e-9).unwrap_or(0)
    }

    /// The keys either side of `t`, as "between keys[2] (1.20 s) and keys[3] (1.50 s)".
    pub fn keys_around(&self, t: f64) -> String {
        let name = |i: usize| format!("keys[{i}] ({:.2} s)", self.keys[i].t);
        match self.keys.iter().position(|k| k.t > t + 1e-9) {
            Some(0) => format!("before {}", name(0)),
            Some(j) => format!("between {} and {}", name(j - 1), name(j)),
            None => format!("after {}", name(self.keys.len() - 1)),
        }
    }

    /// The joint values and placement at `t`, before follow-through, idle and goals.
    fn base(&self, si: usize, t: f64) -> Option<(Vec<f64>, DVec3, DQuat)> {
        let k0 = self.keys[0].hands[si].as_ref()?;
        let mut values = k0.values.clone();
        let mut wrist = k0.wrist;
        let mut rot = k0.rot;
        for w in self.keys.windows(2) {
            let (a, b) = (w[0].hands[si].as_ref()?, w[1].hands[si].as_ref()?);
            for (j, s) in w[1].steps[si].iter().enumerate() {
                let d = b.values[j] - a.values[j];
                if d != 0.0 {
                    values[j] += d * s.at(t);
                }
            }
            let r = w[1].place_steps[si].at(t);
            if r != 0.0 {
                wrist += (b.wrist - a.wrist) * r;
                let mut delta = a.rot.inverse() * b.rot;
                if delta.w < 0.0 {
                    delta = -delta;
                }
                rot *= DQuat::from_scaled_axis(delta.to_scaled_axis() * r);
            }
        }
        Some((values, wrist, rot))
    }

    /// The hand's orientation (forearm plus wrist joints), for follow-through.
    fn hand_rot(&self, si: usize, t: f64) -> (DQuat, DVec3) {
        let (values, wrist, rot) = self.base(si, t).unwrap();
        let idx = |k: &str| PARAMS.iter().position(|p| p.key == k).unwrap();
        let (twist, dev, flex) = (values[idx("twist")], values[idx("wrist_dev")], values[idx("wrist_flex")]);
        let mut local = DQuat::from_rotation_y(twist.to_radians()) * DQuat::from_rotation_z(-dev.to_radians()) * DQuat::from_rotation_x(flex.to_radians());
        if self.ids[si].side == Side::Left {
            // The left hand is built mirrored in x: M R M keeps x rotations and flips y and z.
            local = DQuat::from_xyzw(local.x, -local.y, -local.z, local.w);
        }
        (rot * local, wrist)
    }

    /// The hand's rate of turning toward the palm (rad/s), counting a move of the hand toward
    /// its palm side as a turn about the knuckles.
    fn flex_rate(&self, si: usize, t: f64) -> f64 {
        let h = 1.0 / 240.0;
        let (q0, p0) = self.hand_rot(si, t - h);
        let (q1, p1) = self.hand_rot(si, t + h);
        let mut d = q0.inverse() * q1;
        if d.w < 0.0 {
            d = -d;
        }
        let w = d.to_scaled_axis() / (2.0 * h);
        let v = q0.inverse() * ((p1 - p0) / (2.0 * h));
        w.x + v.z / 0.07
    }

    /// How far relaxed fingers trail the hand (degrees, at the knuckle): a damped finger on a
    /// turning hand, by convolution over the last 0.6 s.
    fn follow_through(&self, si: usize, t: f64) -> f64 {
        if self.follow == 0.0 {
            return 0.0;
        }
        let (w, z) = (std::f64::consts::TAU * 4.5, 0.5);
        let wd = w * (1.0 - z * z as f64).sqrt();
        let dh = |s: f64| (-z * w * s).exp() * ((wd * s).cos() - z * w / wd * (wd * s).sin());
        let lag = integrate(0.0, 0.6, |s| dh(s) * self.flex_rate(si, t - s));
        let deg = -lag.to_degrees() * self.follow;
        25.0 * (deg / 25.0).tanh()
    }

    /// A hand as a pose file's hand: every joint value and the placement.
    /// `forearm` is the forearm's length (the person's size times the default).
    fn hand_value(values: &[f64], wrist: DVec3, rot: DQuat, forearm: f64) -> Value {
        let mut hand = json!({});
        for (j, p) in PARAMS.iter().enumerate() {
            set_path(&mut hand, p.key, json!(r6(values[j])));
        }
        hand["place"] = place_json(wrist, rot, forearm);
        hand
    }

    /// Hand `si` at `t` with idle movement and joint limits, and with follow-through when
    /// `follow` (before collisions and goals).
    fn posed(&self, si: usize, t: f64, follow: bool) -> (Vec<f64>, DVec3, DQuat) {
        let ch = self.chan[si];
        let (mut values, mut wrist, rot) = self.base(si, t).unwrap();
        let drag = if follow { self.follow_through(si, t) } else { 0.0 };
        // Gripping fingers hold still: no idle or follow-through while a grip holds them.
        let mut quiet = vec![1.0; PARAMS.len()];
        for h in &self.holds {
            for (who, hand) in [h.leader, h.follower].into_iter().enumerate() {
                if hand == si {
                    let w = h.weight(t);
                    // The palm's hollow moves the ring and pinky knuckles: it holds still too.
                    let cup = PARAMS.iter().position(|p| p.key == "palm_cup").filter(|_| !h.closing[who].is_empty());
                    for &j in h.closing[who].iter().chain(cup.iter()) {
                        quiet[j] = f64::min(quiet[j], 1.0 - w);
                    }
                }
            }
        }
        for (j, p) in PARAMS.iter().enumerate() {
            let (digit, _) = group_of(p.key);
            let k = match (digit, p.key.split_once('.').map(|x| x.1)) {
                (Some(0), Some("mcp_flex")) => 0.3,
                (Some(d), Some("mcp_flex")) if d > 0 => 1.0,
                (Some(d), Some("pip_flex")) if d > 0 => 0.6,
                (Some(d), Some("dip_flex")) if d > 0 => 0.4,
                _ => 0.0,
            };
            if k != 0.0 {
                // Curled fingers are held, not loose.
                let curl = (values[j] / 90.0).clamp(0.0, 1.0);
                values[j] += k * drag * (1.0 - 0.7 * curl) * quiet[j];
            }
            values[j] += self.idle * idle(self.seed, ch, j, p.key, t) * quiet[j];
            // A held grip keeps its solved values exactly, even near a limit.
            let hard = values[j].clamp(p.min, p.max);
            values[j] = hard + (soft_clamp(values[j], p.min, p.max) - hard) * quiet[j];
        }
        for (a, axis) in [(0, DVec3::X), (1, DVec3::Y), (2, DVec3::Z)] {
            wrist += axis * self.idle * 0.0012 * noise(self.seed, ch * 100 + 90 + a, t, 0.2);
        }
        (values, wrist, rot)
    }

    /// Hand `si` at `t` as `posed`; while a grip holds it as the follower, its hand moves with
    /// the leader's (the leader's frame times the grip's offset) and its forearm trails: the
    /// elbow follows where the hand's own wrist angles would put it through a critically damped
    /// spring, and the wrist bends to make up the difference.
    fn attached(&self, si: usize, t: f64, follow: bool) -> (Vec<f64>, DVec3, DQuat) {
        self.attached_at(si, t, follow, 0)
    }

    fn attached_at(&self, si: usize, t: f64, follow: bool, depth: usize) -> (Vec<f64>, DVec3, DQuat) {
        let (mut values, wrist, rot) = self.posed(si, t, follow);
        let Some(h) = self.holds.iter().find(|h| h.follower == si && h.weight(t) > 0.0).filter(|_| depth < 4) else { return (values, wrist, rot) };
        let w = h.weight(t);
        let side = self.ids[si].side;
        let f = self.held_frame(h, t, depth);
        let wn = std::f64::consts::TAU * TRAIL_HZ;
        let span = 8.0 / wn;
        let kernel = |s: f64| wn * wn * s * (-wn * s).exp();
        let (m, half) = (span / 2.0, span / 2.0);
        let mut elbow = DVec3::ZERO;
        let mut norm = 0.0;
        for (x, gw) in GL_X.iter().zip(GL_W) {
            for s in [m - half * x, m + half * x] {
                let k = gw * kernel(s);
                elbow += self.elbow_of(h, t - s, depth) * k;
                norm += k;
            }
        }
        let d = (elbow / norm - f.translation).normalize_or(f.matrix3.y_axis * -1.0);
        let mut v = DVec3::new(d.dot(f.matrix3.x_axis), d.dot(f.matrix3.y_axis), d.dot(f.matrix3.z_axis));
        if side == Side::Left {
            v.x = -v.x;
        }
        let trail = [("wrist_dev", v.x.clamp(-1.0, 1.0).asin().to_degrees()), ("wrist_flex", v.z.atan2(-v.y).to_degrees())];
        for (k, x) in trail {
            let j = PARAMS.iter().position(|p| p.key == k).unwrap();
            values[j] = soft_clamp(values[j] + (x - values[j]) * w, PARAMS[j].min, PARAMS[j].max);
        }
        let rot = DQuat::from_mat3(&f.matrix3.into()) * wrist_rot(side, &values).inverse();
        (values, f.translation, rot)
    }

    /// A held follower's hand frame at `t`: its own keyed frame, blended into the leader's frame
    /// times the grip's offset as the grip joins them.
    fn held_frame(&self, h: &Hold, t: f64, depth: usize) -> DAffine3 {
        let (v, wrist, rot) = self.posed(h.follower, t, false);
        let own = hand_frame(self.ids[h.follower].side, &v, wrist, rot);
        let (lv, lw, lr) = self.attached_at(h.leader, t, false, depth + 1);
        let leader = hand_frame(self.ids[h.leader].side, &lv, lw, lr);
        let mut offset = h.offsets[0];
        for (n, k) in (h.first + 1..=h.last).enumerate() {
            offset = blend(offset, h.offsets[n + 1], self.keys[k].place_steps[h.follower].at(t).clamp(0.0, 1.0));
        }
        blend(own, leader * offset, h.weight(t))
    }

    /// The length of hand `si`'s forearm: its person's size times the default.
    fn forearm(&self, si: usize) -> f64 {
        FOREARM * pose::person_scale(self.people.as_ref(), &self.ids[si].person).unwrap_or(1.0)
    }

    /// Where a held follower's elbow would be at `t` with its own wrist angles.
    fn elbow_of(&self, h: &Hold, t: f64, depth: usize) -> DVec3 {
        let f = self.held_frame(h, t, depth);
        let (v, _, _) = self.posed(h.follower, t, false);
        let idx = |k: &str| PARAMS.iter().position(|p| p.key == k).unwrap();
        f.translation + f.matrix3 * forearm_dir(self.ids[h.follower].side, v[idx("wrist_dev")], v[idx("wrist_flex")]) * self.forearm(h.follower)
    }

    /// How far collisions push hand `si` at `t`; a held follower moves with its leader's push.
    fn push_of(&self, si: usize, t: f64) -> DVec3 {
        let own = self.track.at(si, t);
        match self.holds.iter().find(|h| h.follower == si && h.weight(t) > 0.0) {
            Some(h) => own.lerp(self.push_of(h.leader, t), h.weight(t)),
            None => own,
        }
    }

    /// Every hand at `t` before collisions and goals, and its wrist: the motion the collisions
    /// push. Follow-through (slow to compute) only for hands near another.
    fn free_hands(&self, t: f64) -> Result<(Vec<HandGeom>, Vec<DVec3>), String> {
        let build = |si: usize, follow: bool| -> Result<(HandGeom, DVec3), String> {
            let (values, wrist, rot) = self.attached(si, t, follow);
            let mut hp: pose::HandPose = serde_json::from_value(Self::hand_value(&values, wrist, rot, self.forearm(si))).map_err(|e| e.to_string())?;
            hp.scale = pose::person_scale(self.people.as_ref(), &self.ids[si].person);
            Ok((crate::rig::build_hand(&self.ids[si], &hp, Some(1), None)?, wrist))
        };
        let (mut geoms, mut wrists): (Vec<HandGeom>, Vec<DVec3>) = (0..self.ids.len()).map(|si| build(si, false)).collect::<Result<Vec<_>, _>>()?.into_iter().unzip();
        if self.follow != 0.0 {
            let boxes: Vec<_> = geoms.iter().map(collide::bounds).collect();
            for si in 0..geoms.len() {
                let near = (0..geoms.len()).any(|o| o != si && collide::boxes_near(boxes[si], boxes[o], 0.04));
                if near {
                    (geoms[si], wrists[si]) = build(si, true)?;
                }
            }
        }
        Ok((geoms, wrists))
    }

    /// Whether hand `si` collides at `t`: the setting of its latest key whose move has begun.
    fn collides_at(&self, si: usize, t: f64) -> bool {
        let k = (1..self.keys.len()).rev().find(|k| self.keys[*k].named[si] && self.keys[*k].place_steps[si].begin() <= t).unwrap_or(0);
        self.keys[k].collide[si]
    }

    /// The parts of two hands that the goals held at `t` join (every part any of their goals
    /// names, on each hand): those may touch and overlap.
    pub fn joined_parts(&self, t: f64, hands: &[HandGeom]) -> Vec<collide::Exempt> {
        let (held, new, dropped) = self.goals_at(t);
        let goals = held.iter().chain(new.iter().chain(&dropped).filter(|g| g.1 > 0.0).map(|g| &g.0));
        let mut out: Vec<collide::Exempt> = Vec::new();
        for g in goals {
            let Some([a, b]) = g.get("touch").and_then(Value::as_array).and_then(|n| Some([n.first()?.as_str()?, n.get(1)?.as_str()?])) else { continue };
            let (Some((ia, ra)), Some((ib, rb))) = (pose::split_hand(a), pose::split_hand(b)) else { continue };
            let (Some(mut ha), Some(mut hb)) = (self.ids.iter().position(|h| *h == ia), self.ids.iter().position(|h| *h == ib)) else { continue };
            if ha == hb {
                continue;
            }
            let (mut ma, mut mb) = (parts_named(&hands[ha], ra), parts_named(&hands[hb], rb));
            if ha > hb {
                (ha, hb, ma, mb) = (hb, ha, mb, ma);
            }
            match out.iter_mut().find(|e| (e.a, e.b) == (ha, hb)) {
                Some(collide::Exempt { parts: Some((xa, xb)), .. }) => {
                    xa.iter_mut().zip(&ma).for_each(|(x, m)| *x |= *m);
                    xb.iter_mut().zip(&mb).for_each(|(x, m)| *x |= *m);
                }
                _ => out.push(collide::Exempt { a: ha, b: hb, parts: Some((ma, mb)) }),
            }
        }
        // Two hands taking, holding or letting go of each other's grip touch everywhere.
        for h in self.holds.iter().filter(|h| t >= h.exempt.0 && t <= h.exempt.1) {
            out.push(collide::Exempt { a: h.leader.min(h.follower), b: h.leader.max(h.follower), parts: None });
        }
        out
    }

    /// Runs the collisions between the hands over the whole clip (see `collide`).
    fn collisions(&self) -> Result<collide::Track, String> {
        let first = self.keys.iter().flat_map(|k| k.place_steps.iter().map(|s| s.begin())).fold(0.0_f64, f64::min);
        let last = self.keys.last().unwrap().t.max(self.duration);
        collide::simulate(first - 0.25, last + 1.0, |t| {
            let (hands, wrists) = self.free_hands(t)?;
            let on = (0..self.ids.len()).map(|si| self.collides_at(si, t)).collect();
            let exempt = self.joined_parts(t, &hands);
            Ok(collide::Moment { hands, wrists, on, exempt })
        })
    }

    /// How far collisions push each hand at `t` (meters, in the clip's hand order).
    pub fn pushes(&self, t: f64) -> Vec<DVec3> {
        (0..self.ids.len()).map(|si| self.track.at(si, t)).collect()
    }

    /// The pose and active goals at `t` seconds.
    pub fn sample(&self, t: f64) -> Result<Frame, String> {
        let mut hands = serde_json::Map::new();
        for (si, id) in self.ids.iter().enumerate() {
            let (values, mut wrist, rot) = self.attached(si, t, true);
            wrist += self.push_of(si, t);
            hands.insert(id.name(), Self::hand_value(&values, wrist, rot, self.forearm(si)));
        }
        let mut pose = json!({ "version": 1, "hands": Value::Object(hands) });
        if let Some(c) = self.camera_at(t) {
            pose["camera"] = c;
        }
        if let Some(s) = &self.size {
            pose["size"] = s.clone();
        }
        if let Some(l) = &self.look {
            pose["look"] = l.clone();
        }
        if let Some(p) = &self.people {
            pose["people"] = p.clone();
        }

        // Goals: held through a transition when both keys have them, ramped in near a key's
        // arrival when new, ramped out at a transition's start when dropped.
        let (held, new, dropped) = self.goals_at(t);
        // Grips are not re-solved: their keys' solved fingers and the follower moving with the
        // leader keep them closed. A grip counts as held once its key is reached.
        let is_grip = |g: &Value| g.get("grip").is_some();
        let (grips, held): (Vec<Value>, Vec<Value>) = held.into_iter().partition(is_grip);
        let reached = |g: &Value| self.holds.iter().any(|h| grip_id(g) == Some(h.id.clone()) && t >= self.keys[h.first].t - 1e-9 && t < h.release);
        let grips: Vec<Value> = grips.into_iter().chain(new.iter().map(|g| g.0.clone())).filter(|g| is_grip(g) && reached(g)).collect();
        let new: Vec<(Value, f64)> = new.into_iter().filter(|g| !is_grip(&g.0)).collect();
        let dropped: Vec<(Value, f64)> = dropped.into_iter().filter(|g| !is_grip(&g.0)).collect();
        // Re-solving goals keeps a hand's own parts apart, and the two hands apart only while
        // a goal joins them (overlaps between hands are handled below).
        let gripping = self.holds.iter().any(|h| t >= h.exempt.0 && t <= h.exempt.1);
        let joined = gripping
            || held.iter().chain(new.iter().chain(&dropped).map(|g| &g.0)).any(|g| {
                let names: Vec<&str> = g.get("touch").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).collect();
                let hands: Vec<String> = names.iter().map(|n| pose::hand_of(n)).collect();
                hands.windows(2).any(|w| w[0] != w[1])
            });
        let solve = |pose: &Value, goals: &[Value], own_hand: bool| -> Result<Value, String> {
            if goals.is_empty() {
                return Ok(pose.clone());
            }
            let mut p = pose.clone();
            p["goals"] = json!(goals);
            p["version"] = json!(1);
            let (mut out, _) = crate::solve::solve_goals_in(&p, false, own_hand)?;
            out.as_object_mut().unwrap().remove("goals");
            Ok(out)
        };
        let goals_of = |v: &[(Value, f64)]| v.iter().map(|g| g.0.clone()).collect::<Vec<_>>();
        let base = solve(&pose, &held, !joined)?;
        let mut out = base.clone();
        let mut active = held.clone();
        for extra in [new, dropped] {
            let Some(w) = extra.first().map(|g| g.1).filter(|w| *w > 0.0) else { continue };
            let all: Vec<Value> = held.iter().cloned().chain(goals_of(&extra)).collect();
            let full = solve(&base, &all, !joined)?;
            out = add(&out, &scale(&sub(&full, &base), w));
            if w >= 0.999 {
                active.extend(goals_of(&extra));
            }
        }
        // A hand's fingers must not pass through each other or its palm mid-move (a thumb
        // through the fingers as a fist opens): where they would, push the digits involved
        // apart, keeping the held goals. Overlaps between the two hands are left to the author
        // (`check` reports them): pushed apart each frame, a finger passing through the other
        // hand flips from one side of it to the other.
        out["version"] = json!(1);
        let solved = crate::solve_value(&out)?;
        let report = crate::query::analyze(&solved.pose, &solved.hands)?;
        let side_of = |label: &str| pose::hand_of(label);
        let mut vary: Vec<String> = Vec::new();
        for p in report.contacts.iter().chain(&report.penetrations).filter(|p| p.gap_mm < -0.9 && side_of(&p.a) == side_of(&p.b)) {
            for label in [&p.a, &p.b] {
                let Some((id, rest)) = pose::split_hand(label) else { continue };
                let (side, part) = (id.name(), rest.split('.').next().unwrap_or(""));
                if DIGITS.contains(&part) {
                    let pat = format!("{side}.{part}.*");
                    if !vary.contains(&pat) {
                        vary.push(pat);
                    }
                }
            }
        }
        if !vary.is_empty() {
            let goals: Vec<Value> = active.iter().cloned().chain(std::iter::once(json!({ "vary": vary }))).collect();
            out = solve(&out, &goals, !joined)?;
        }
        let mut pose = round_numbers(&out);
        pose["version"] = json!(1);
        active.extend(grips);
        Ok(Frame { pose, goals: active })
    }

    /// The clip's camera, moved by each key that sets one, on that key's step.
    fn camera_at(&self, t: f64) -> Option<Value> {
        let mut cam = self.camera.clone()?;
        let mut prev = cam.clone();
        for k in &self.keys[1..] {
            let Some(c) = &k.camera else { continue };
            let mut target = prev.clone();
            pose::merge(&mut target, c.clone());
            cam = add(&cam, &scale(&sub(&target, &prev), k.cam_step.at(t)));
            prev = target;
        }
        Some(cam)
    }

    /// (held, ramping in, ramping out), the ramps with their weight.
    fn goals_at(&self, t: f64) -> (Vec<Value>, Vec<(Value, f64)>, Vec<(Value, f64)>) {
        // The latest key whose step has begun.
        let mut i = 0;
        for (j, k) in self.keys.iter().enumerate().skip(1) {
            if t >= k.begin() {
                i = j;
            }
        }
        let k = &self.keys[i];
        if i == 0 || t >= k.t {
            return (k.goals.clone(), Vec::new(), Vec::new());
        }
        let p = &self.keys[i - 1];
        let begin = k.begin();
        let ramp = ((k.t - begin) / 2.0).min(0.25).max(1e-3);
        let held: Vec<Value> = k.goals.iter().filter(|g| p.goals.contains(g)).cloned().collect();
        let win = smooth((t - (k.t - ramp)) / ramp);
        let wout = 1.0 - smooth((t - begin) / ramp);
        let new = k.goals.iter().filter(|g| !p.goals.contains(g)).map(|g| (g.clone(), win)).collect();
        let dropped = p.goals.iter().filter(|g| !k.goals.contains(g)).map(|g| (g.clone(), wout)).collect();
        (held, new, dropped)
    }
}

/// Every run of keys holding a grip. Their timing changes: in the key a grip starts in, the
/// gripping fingers close over the last `GRIP_CLOSE` s of the move, without overshoot (the hands
/// meet first); in the key after the run, they open over `GRIP_OPEN` s at the start of the move
/// and the two hands start apart `GRIP_PART_DELAY` s later.
fn holds_of(keys: &mut [Key], ids: &[HandId]) -> Result<Vec<Hold>, String> {
    let mut out = Vec::new();
    for i in 0..keys.len() {
        for g in keys[i].goals.clone() {
            let Some(id) = grip_id(&g) else { continue };
            let has = |k: &Key| k.goals.iter().any(|q| grip_id(q).as_ref() == Some(&id));
            if i > 0 && has(&keys[i - 1]) {
                continue;
            }
            let mut last = i;
            while last + 1 < keys.len() && has(&keys[last + 1]) {
                last += 1;
            }
            let at = |e: String| format!("keys[{i}]: {e}");
            let goal: pose::Goal = serde_json::from_value(g.clone()).map_err(|e| at(e.to_string()))?;
            let (la, lb) = crate::grip::pair(&goal).ok_or_else(|| at("grip: name two hands, e.g. [\"ana.right\", \"ben.right\"]".into()))?;
            let find = |h: &HandId| ids.iter().position(|x| x == h).ok_or_else(|| at(format!("grip: there is no {h} hand in the clip")));
            let (leader, follower) = (find(&la)?, find(&lb)?);
            if out.iter().any(|h: &Hold| h.follower == follower && h.first <= last && i <= h.last) {
                return Err(at(format!("grip: the {lb} hand already follows another hand here; a hand follows one grip at a time")));
            }
            let digits = crate::grip::closing(&goal).map_err(at)?;
            let closing = digits.map(|ds| (0..PARAMS.len()).filter(|j| ds.iter().any(|d| PARAMS[*j].key.starts_with(&format!("{d}.")))).collect::<Vec<_>>());
            let offsets = (i..=last).map(|k| keys[k].frames[leader].inverse() * keys[k].frames[follower]).collect();
            // The gripping digits close and open together (no cascade), timed by the hand's move.
            // A grip without them (a high five) is a plain touch.
            let fingers = closing.iter().any(|c| !c.is_empty());
            if i > 0 && fingers {
                // The hands arrive GRIP_CLOSE early; the fingers then close, done a little
                // before the key so they have settled when it arrives.
                for (who, hand) in [leader, follower].into_iter().enumerate() {
                    let k = &mut keys[i];
                    let end = k.place_steps[hand].start + k.place_steps[hand].dur;
                    for s in k.steps[hand].iter_mut().chain(std::iter::once(&mut k.place_steps[hand])) {
                        s.dur = (s.dur - GRIP_CLOSE).max(s.dur.min(0.15));
                    }
                    for &j in &closing[who] {
                        let s = &mut k.steps[hand][j];
                        (s.start, s.dur) = (end - GRIP_CLOSE, GRIP_CLOSE - 0.1);
                        s.spring = Spring { w: f64::INFINITY, zeta: 1.0 };
                    }
                }
            }
            let mut release = f64::INFINITY;
            if last + 1 < keys.len() {
                let k = &mut keys[last + 1];
                for (who, hand) in [leader, follower].into_iter().enumerate() {
                    let begin = k.place_steps[hand].start;
                    release = release.min(begin);
                    if !fingers {
                        continue;
                    }
                    for (j, s) in k.steps[hand].iter_mut().enumerate() {
                        if closing[who].contains(&j) {
                            (s.start, s.dur) = (begin, s.dur.min(GRIP_OPEN));
                            s.spring = Spring { w: f64::INFINITY, zeta: 1.0 };
                        } else {
                            s.start += GRIP_PART_DELAY;
                            s.dur = (s.dur - GRIP_PART_DELAY).max(0.15);
                        }
                    }
                    let s = &mut k.place_steps[hand];
                    s.start += GRIP_PART_DELAY;
                    s.dur = (s.dur - GRIP_PART_DELAY).max(0.15);
                }
            }
            let join = if i == 0 {
                (f64::NEG_INFINITY, f64::NEG_INFINITY)
            } else {
                let s = &keys[i].place_steps[follower];
                let arrive = s.start + s.dur + s.spring.lag();
                let r = (0.4 * s.dur).clamp(0.05, 0.25);
                (arrive - r, arrive)
            };
            let part = match keys.get(last + 1) {
                Some(k) => {
                    let s = &k.place_steps[follower];
                    (s.begin(), s.begin() + (0.5 * s.dur).clamp(0.08, 0.35))
                }
                None => (f64::INFINITY, f64::INFINITY),
            };
            let exempt = (if i == 0 { f64::NEG_INFINITY } else { keys[i].begin() - 0.05 }, part.1 + 0.3);
            out.push(Hold { id, leader, follower, first: i, last, offsets, join, part, release, exempt, closing });
        }
    }
    Ok(out)
}

/// Merges a key over the previous key. A key-level `from` takes the whole gesture (its hands,
/// placements and goals) for the hands it has; a hand written with `from` or `mirror` starts
/// fresh (keeping the previous placement unless it mirrors); a placement written replaces the
/// previous one; everything else overrides value by value. Goals are the key's own.
fn merge_key(prev: Option<&Value>, key: Value) -> Result<Value, String> {
    let Some(prev) = prev else { return Ok(key) };
    let mut base = prev.clone();
    let mut key = key;
    base.as_object_mut().unwrap().remove("goals");
    if let Some(name) = key.as_object_mut().and_then(|o| o.remove("from")) {
        let name = name.as_str().ok_or("\"from\" must be a gesture name")?;
        let g = pose::gesture_value(name)?;
        for (side, gh) in g.get("hands").and_then(Value::as_object).cloned().unwrap_or_default() {
            base["hands"][&side] = gh;
        }
        let mut goals = g.get("goals").and_then(Value::as_array).cloned().unwrap_or_default();
        goals.extend(key.get("goals").and_then(Value::as_array).cloned().unwrap_or_default());
        if !goals.is_empty() {
            key["goals"] = json!(goals);
        }
        if let Some(c) = g.get("camera").filter(|_| key.get("camera").is_none()) {
            base["camera"] = c.clone();
        }
    }
    let key_hands = key.get("hands").and_then(Value::as_object).cloned().unwrap_or_default();
    for (side, kh) in key_hands {
        let fresh = kh.get("from").is_some() || kh.get("mirror").is_some();
        let own_place = ["place", "elbow", "forearm_rotation_deg"].iter().any(|k| kh.get(*k).is_some());
        let slot = &mut base["hands"][&side];
        if fresh {
            let keep = if kh.get("mirror").is_none() && !own_place { slot.get("place").cloned() } else { None };
            *slot = json!({});
            if let Some(p) = keep {
                slot["place"] = p;
            }
        } else if own_place {
            if let Some(o) = slot.as_object_mut() {
                for k in ["place", "elbow", "forearm_rotation_deg"] {
                    o.remove(k);
                }
            }
        }
        if slot.is_null() {
            *slot = json!({});
        }
        pose::merge(slot, kh);
    }
    for (k, v) in key.as_object().cloned().unwrap_or_default() {
        if k != "hands" {
            base[&k] = v;
        }
    }
    Ok(base)
}

fn place_json(wrist: DVec3, rot: DQuat, forearm: f64) -> Value {
    let elbow = wrist - rot * DVec3::new(0.0, forearm, 0.0);
    let (x, y, z) = rot.to_euler(EulerRot::XYZ);
    json!({
        "elbow": [r6(elbow.x), r6(elbow.y), r6(elbow.z)],
        "forearm_rotation_deg": [r6(x.to_degrees()), r6(y.to_degrees()), r6(z.to_degrees())],
    })
}

/// The capsules of `hand` a site or segment name (after the hand) belongs to: its whole digit,
/// the forearm, or for a palm site the palm.
fn parts_named(hand: &HandGeom, rest: &str) -> Vec<bool> {
    let part = rest.split('.').next().unwrap_or("");
    let digit = Digit::ALL.into_iter().find(|d| d.name() == part);
    hand.capsules
        .iter()
        .map(|c| match (digit, part) {
            (Some(d), _) => c.digit == Some(d),
            (None, "forearm") => c.seg == Seg::Arm,
            _ => c.part == "palm" || (c.digit == Some(Digit::Thumb) && c.seg == Seg::Metacarpal),
        })
        .collect()
}

/// Smoothstep on [0, 1].
fn smooth(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Keeps a value inside its limits without a kink: within 4 degrees of a limit it eases in.
fn soft_clamp(x: f64, lo: f64, hi: f64) -> f64 {
    let m = 4.0_f64.min((hi - lo) / 4.0);
    if x > hi - m {
        hi - m * (-(x - (hi - m)) / m).exp()
    } else if x < lo + m {
        lo + m * (-((lo + m) - x) / m).exp()
    } else {
        x
    }
}

fn hash(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e3779b97f4a7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}

fn unit(seed: u64, a: usize, b: usize) -> f64 {
    (hash(seed.wrapping_mul(1_000_003) ^ hash((a as u64) << 8 | b as u64)) >> 11) as f64 / (1u64 << 53) as f64
}

/// Smooth seeded noise in about [-1, 1]: three slow sines, around `hz`.
fn noise(seed: u64, channel: usize, t: f64, hz: f64) -> f64 {
    (0..3)
        .map(|k| {
            let f = hz * (0.6 + 0.9 * unit(seed, channel, k * 2));
            let ph = std::f64::consts::TAU * unit(seed, channel, k * 2 + 1);
            (std::f64::consts::TAU * f * t + ph).sin() / 1.8
        })
        .sum()
}

/// The idle layer for one joint value, degrees.
fn idle(seed: u64, si: usize, j: usize, key: &str, t: f64) -> f64 {
    let own = noise(seed, si * 100 + j, t, 0.3);
    let shared = noise(seed, si * 100 + 80, t, 0.25);
    match key {
        "wrist_flex" => 1.4 * own,
        "wrist_dev" => 0.8 * own,
        "twist" => 1.0 * own,
        "palm_cup" => 0.6 * own.abs(),
        k if k.ends_with("mcp_flex") => 1.0 * (0.5 * own + 0.6 * shared),
        k if k.ends_with("pip_flex") => 0.8 * (0.5 * own + 0.6 * shared),
        k if k.ends_with("mcp_abd") => 0.5 * own,
        "thumb.cmc_abd" => 0.8 * own,
        _ => 0.0,
    }
}

fn map_numbers(a: &Value, b: &Value, f: &impl Fn(f64, f64) -> f64) -> Value {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => json!(f(x.as_f64().unwrap(), y.as_f64().unwrap())),
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => Value::Array(x.iter().zip(y).map(|(p, q)| map_numbers(p, q, f)).collect()),
        (Value::Object(x), Value::Object(y)) => {
            let mut o = x.clone();
            for (k, q) in y {
                let v = match x.get(k) {
                    Some(p) => map_numbers(p, q, f),
                    None => q.clone(),
                };
                o.insert(k.clone(), v);
            }
            Value::Object(o)
        }
        (_, q) => q.clone(),
    }
}

fn add(a: &Value, b: &Value) -> Value {
    map_numbers(a, b, &|x, y| x + y)
}

fn sub(a: &Value, b: &Value) -> Value {
    map_numbers(a, b, &|x, y| x - y)
}

fn scale(a: &Value, w: f64) -> Value {
    map_numbers(a, a, &|x, _| x * w)
}

fn round_numbers(a: &Value) -> Value {
    map_numbers(a, a, &|x, _| r6(x))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn euler_matches_the_rig() {
        let q = DQuat::from_rotation_x(0.3) * DQuat::from_rotation_y(-0.7) * DQuat::from_rotation_z(1.1);
        let (x, y, z) = q.to_euler(EulerRot::XYZ);
        assert!((x - 0.3).abs() < 1e-9 && (y + 0.7).abs() < 1e-9 && (z - 1.1).abs() < 1e-9);
    }

    #[test]
    fn steps_start_at_zero_and_settle_at_one() {
        for (shape, spring) in [(Shape::MinJerk, Spring { w: 25.0, zeta: 0.7 }), (Shape::MinJerk, Spring { w: f64::INFINITY, zeta: 1.0 }), (Shape::Linear, Spring { w: 40.0, zeta: 0.45 })] {
            let s = Step { start: 1.0, dur: 0.5, anticipate: 0.1, ant_dur: 0.15, shape, spring };
            assert_eq!(s.at(0.8), 0.0);
            assert!((s.at(5.0) - 1.0).abs() < 1e-9);
            // Continuous: no jump anywhere.
            let mut last = 0.0;
            for i in 0..=600 {
                let x = s.at(0.8 + i as f64 * 0.005);
                assert!((x - last).abs() < 0.06, "jump at {}", 0.8 + i as f64 * 0.005);
                last = x;
            }
        }
    }

    #[test]
    fn anticipation_goes_the_other_way_first() {
        let s = Step { start: 1.0, dur: 0.5, anticipate: 0.1, ant_dur: 0.2, shape: Shape::MinJerk, spring: Spring { w: f64::INFINITY, zeta: 1.0 } };
        assert!((s.at(1.0) + 0.1).abs() < 1e-9);
    }
}
