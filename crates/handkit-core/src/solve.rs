//! `handkit solve`: changes joint values (and, when asked, placement) until the file's goals
//! hold, without fingers passing through each other, and writes the values back into the file.
//! Damped least squares (Levenberg-Marquardt) with numeric derivatives; joint limits are hard
//! bounds; a small pull toward the starting pose keeps unneeded joints where they were.

use serde_json::{Value, json};

use crate::goals::{self, GoalStatus};
use crate::params::PARAMS;
use crate::pose::resolve;
use crate::query::{self, seg_dist};
use crate::{Solved, solve_value};

/// Overlap (m) the solver tolerates before pushing segments apart.
const OVERLAP_SLACK: f64 = 0.0005;
/// Per pass: the pull toward the starting pose per normalized unit (10 deg, 2 cm, ...) and the
/// weight against overlaps. The first pass may pass fingers through each other to reach the
/// right arrangement (only the result matters); the second pushes them apart; the third closes
/// the last fraction of a millimeter.
const STAGES: [(f64, f64); 3] = [(1e-3, 0.0), (1e-3, 1.0), (2e-4, 1.0)];
/// Meters of residual per 10 deg that a varied dip_flex strays from 2/3 of its pip_flex.
const DIP_COUPLING_WEIGHT: f64 = 0.001;
/// Meters of residual for a joint at its limit, fading to zero 15% of its range inside it, so
/// the solver avoids awkward extremes when another way works. Counted only beyond where the
/// joint started, so a value the file chose is never pushed away.
const COMFORT_WEIGHT: f64 = 0.004;
const COMFORT_BAND: f64 = 0.15;
const MAX_ITERS: usize = 150;

#[derive(Clone)]
struct Param {
    label: String,
    path: Vec<Value>,
    scale: f64,
    lo: f64,
    hi: f64,
    start: f64,
    /// A dip_flex the file leaves to the 2/3 coupling.
    coupled: bool,
}

pub struct SolveReport {
    pub iterations: usize,
    pub goals: Vec<GoalStatus>,
    pub changes: Vec<(String, f64, f64)>,
    pub penetrations: usize,
    pub varied: usize,
}

impl SolveReport {
    pub fn text(&self) -> String {
        let mut s = String::new();
        let met = self.goals.iter().filter(|g| g.met).count();
        s.push_str(&format!("goals met: {met} of {} ({} values varied, {} iterations)\n", self.goals.len(), self.varied, self.iterations));
        for g in &self.goals {
            s.push_str(&format!("  {}\n", g.line()));
        }
        s.push_str(&format!("penetrations (overlap > {} mm): {}\n", query::PENETRATION_MM, self.penetrations));
        if self.changes.is_empty() {
            s.push_str("changed: nothing\n");
        } else {
            s.push_str("changed:\n");
            for (k, a, b) in &self.changes {
                s.push_str(&format!("  {k}: {a} -> {b}\n"));
            }
        }
        s
    }
}

pub fn solve_goals(raw: &Value) -> Result<(Value, SolveReport), String> {
    solve_goals_with(raw, true)
}

/// `solve_goals`, optionally without rounding the values it writes (clips solve every frame,
/// where rounding would make the hand jitter).
pub fn solve_goals_with(raw: &Value, round: bool) -> Result<(Value, SolveReport), String> {
    solve_goals_in(raw, round, false)
}

/// `solve_goals_with`; `own_hand` keeps only each hand's own parts apart, not one hand from
/// the other.
pub fn solve_goals_in(raw: &Value, round: bool, own_hand: bool) -> Result<(Value, SolveReport), String> {
    let (resolved, _) = resolve(raw.clone())?;
    let first = solve_value(raw)?;
    if first.pose.goals.is_empty() {
        return Err("the file has no goals; add e.g. \"goals\": [{\"touch\": [\"right.thumb.pad\", \"right.index.pad\"]}]".into());
    }
    let mut work = raw.clone();
    let params = collect_params(raw, &resolved, &first, &mut work)?;
    if params.is_empty() {
        return Err("no values to vary".into());
    }
    // Goals that already hold need no change.
    if goals::statuses(&first)?.iter().all(|g| g.met) {
        let report = SolveReport {
            iterations: 0,
            goals: goals::statuses(&first)?,
            changes: Vec::new(),
            penetrations: first.query()?.penetrations.len(),
            varied: params.len(),
        };
        return Ok((raw.clone(), report));
    }
    let u0: Vec<f64> = params.iter().map(|p| p.start / p.scale).collect();
    let mut u = u0.clone();
    let mut iterations = 0;

    let apply = |u: &[f64]| -> Value {
        let mut v = work.clone();
        for (p, x) in params.iter().zip(u) {
            set(&mut v, &p.path, x * p.scale);
        }
        v
    };
    let clamp = |u: &mut [f64]| {
        for (p, x) in params.iter().zip(u.iter_mut()) {
            *x = x.clamp(p.lo / p.scale, p.hi / p.scale);
        }
    };

    for (reg, overlap) in STAGES {
        let mut lambda = 1e-2;
        for _ in 0..MAX_ITERS {
            iterations += 1;
            let current = solve_value(&apply(&u))?;
            let pairs = near_pairs(&current, own_hand);
            let res = |u: &[f64]| -> Result<Vec<f64>, String> {
                let s = solve_value(&apply(u))?;
                residuals(&s, &pairs, u, &u0, reg, overlap, &params)
            };
            let r = res(&u)?;
            let cost = dot(&r, &r);
            if cost < 1e-14 {
                break;
            }
            let n = u.len();
            let mut jac = vec![vec![0.0; n]; r.len()];
            for i in 0..n {
                let mut up = u.clone();
                let mut h = 1e-3;
                if up[i] + h > params[i].hi / params[i].scale {
                    h = -h;
                }
                up[i] += h;
                let rp = res(&up)?;
                for (row, (a, b)) in jac.iter_mut().zip(rp.iter().zip(&r)) {
                    row[i] = (a - b) / h;
                }
            }
            let mut jtj = vec![vec![0.0; n]; n];
            let mut jtr = vec![0.0; n];
            for (row, ri) in jac.iter().zip(&r) {
                for a in 0..n {
                    jtr[a] += row[a] * ri;
                    for b in 0..n {
                        jtj[a][b] += row[a] * row[b];
                    }
                }
            }
            let mut improved = false;
            while lambda < 1e10 {
                let mut m = jtj.clone();
                for a in 0..n {
                    m[a][a] += lambda * (jtj[a][a] + 1e-9);
                }
                let Some(step) = solve_linear(m, jtr.iter().map(|x| -x).collect()) else {
                    lambda *= 5.0;
                    continue;
                };
                let mut cand: Vec<f64> = u.iter().zip(&step).map(|(a, b)| a + b).collect();
                clamp(&mut cand);
                let rc = res(&cand)?;
                let cc = dot(&rc, &rc);
                if cc < cost {
                    improved = cost - cc > (1e-6 * cost).max(1e-16);
                    u = cand;
                    lambda = (lambda / 3.0).max(1e-7);
                    break;
                }
                lambda *= 5.0;
            }
            if !improved {
                break;
            }
        }
        if std::env::var_os("HANDKIT_DEBUG").is_some() {
            let s = solve_value(&apply(&u))?;
            let g: Vec<String> = goals::statuses(&s)?.iter().map(|g| g.line()).collect();
            let r = residuals(&s, &near_pairs(&s, own_hand), &u, &u0, reg, overlap, &params)?;
            eprintln!("stage reg={reg} overlap={overlap}: iters {iterations}, cost {:.3e}, goals {g:?}", dot(&r, &r));
        }
    }

    // Write what changed (and what the file already had); a value that stayed at its default
    // stays out of the file.
    let mut out = work.clone();
    let mut changes = Vec::new();
    for (p, x) in params.iter().zip(&u) {
        let v = if round { round_for(p, x * p.scale) } else { x * p.scale };
        let changed = (v - round_for(p, p.start)).abs() > 1e-9;
        if changed || get(&work, &p.path).is_some() {
            set(&mut out, &p.path, v);
        }
        if changed {
            changes.push((p.label.clone(), round_for(p, p.start), v));
        }
    }
    let done = solve_value(&out)?;
    let report = SolveReport {
        iterations,
        goals: goals::statuses(&done)?,
        changes,
        penetrations: done.query()?.penetrations.len(),
        varied: params.len(),
    };
    Ok((out, report))
}

fn round_for(p: &Param, x: f64) -> f64 {
    let per = if p.scale >= 1.0 { 10.0 } else if p.scale > 0.1 { 1000.0 } else { 10000.0 };
    (x * per).round() / per + 0.0
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Capsule pairs (by index) that may collide and are within 5 mm of each other (with
/// `own_hand`, only pairs on one hand).
fn near_pairs(s: &Solved, own_hand: bool) -> Vec<(usize, usize)> {
    let caps: Vec<_> = s.hands.iter().flat_map(|h| &h.capsules).collect();
    let mut out = Vec::new();
    for i in 0..caps.len() {
        for j in i + 1..caps.len() {
            let (a, b) = (caps[i], caps[j]);
            if query::collides(a, b) && (!own_hand || a.side == b.side) && seg_dist(a.a, a.b, b.a, b.b) - a.r - b.r < 0.005 {
                out.push((i, j));
            }
        }
    }
    out
}

fn residuals(s: &Solved, pairs: &[(usize, usize)], u: &[f64], u0: &[f64], reg: f64, overlap: f64, params: &[Param]) -> Result<Vec<f64>, String> {
    let mut r = Vec::new();
    for g in &s.pose.goals {
        r.extend(goals::evaluate(g, s)?.0);
    }
    for (p, x) in params.iter().zip(u) {
        if let Some(prefix) = p.label.strip_suffix("dip_flex").filter(|_| p.coupled) {
            let pip = params.iter().zip(u).find(|(q, _)| q.label == format!("{prefix}pip_flex")).map(|(q, y)| y * q.scale);
            let pip = pip.unwrap_or_else(|| pip_value(s, prefix));
            r.push(DIP_COUPLING_WEIGHT * (x * p.scale - pip * 2.0 / 3.0) / 10.0);
        }
        if p.lo.is_finite() && p.hi.is_finite() {
            let extreme = |v: f64| {
                let t = (v - p.lo) / (p.hi - p.lo);
                (COMFORT_BAND - t).max(0.0) + (t - 1.0 + COMFORT_BAND).max(0.0)
            };
            r.push(COMFORT_WEIGHT * (extreme(x * p.scale) - extreme(p.start)).max(0.0) / COMFORT_BAND);
        }
    }
    let caps: Vec<_> = s.hands.iter().flat_map(|h| &h.capsules).collect();
    for &(i, j) in pairs {
        let (a, b) = (caps[i], caps[j]);
        let gap = seg_dist(a.a, a.b, b.a, b.b) - a.r - b.r;
        r.push(overlap * (gap + OVERLAP_SLACK).min(0.0));
    }
    r.extend(u.iter().zip(u0).map(|(a, b)| reg * (a - b)));
    Ok(r)
}

/// pip_flex of "<hand>.<finger>." as the file currently has it.
fn pip_value(s: &Solved, prefix: &str) -> f64 {
    let mut parts = prefix.trim_end_matches('.').split('.');
    let (side, finger) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    let hand = if side == "left" { &s.pose.hands.left } else { &s.pose.hands.right };
    let f = hand.as_ref().and_then(|h| match finger {
        "index" => h.index.as_ref(),
        "middle" => h.middle.as_ref(),
        "ring" => h.ring.as_ref(),
        _ => h.pinky.as_ref(),
    });
    f.and_then(|f| f.pip_flex).unwrap_or(0.0)
}

/// Gaussian elimination with partial pivoting.
fn solve_linear(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    for c in 0..n {
        let p = (c..n).max_by(|&x, &y| a[x][c].abs().total_cmp(&a[y][c].abs()))?;
        if a[p][c].abs() < 1e-18 {
            return None;
        }
        a.swap(c, p);
        b.swap(c, p);
        for r in c + 1..n {
            let f = a[r][c] / a[c][c];
            if f != 0.0 {
                for k in c..n {
                    a[r][k] -= f * a[c][k];
                }
                b[r] -= f * b[c];
            }
        }
    }
    let mut x = vec![0.0; n];
    for c in (0..n).rev() {
        let s: f64 = (c + 1..n).map(|k| a[c][k] * x[k]).sum();
        x[c] = (b[c] - s) / a[c][c];
    }
    Some(x)
}

fn get<'a>(v: &'a Value, path: &[Value]) -> Option<&'a Value> {
    path.iter().try_fold(v, |cur, k| match k {
        Value::String(s) => cur.get(s.as_str()),
        Value::Number(i) => cur.get(i.as_u64()? as usize),
        _ => None,
    })
}

fn set(v: &mut Value, path: &[Value], x: f64) {
    set_value(v, path, json!(x));
}

fn set_value(v: &mut Value, path: &[Value], x: Value) {
    let mut cur = v;
    for (n, k) in path.iter().enumerate() {
        let last = n + 1 == path.len();
        match k {
            Value::String(s) => {
                if !cur.is_object() {
                    *cur = json!({});
                }
                let obj = cur.as_object_mut().unwrap();
                if last {
                    obj.insert(s.clone(), x);
                    return;
                }
                cur = obj.entry(s.clone()).or_insert(json!({}));
            }
            Value::Number(i) => {
                let i = i.as_u64().unwrap() as usize;
                if last {
                    cur[i] = x;
                    return;
                }
                cur = &mut cur[i];
            }
            _ => unreachable!(),
        }
    }
}

const SIDES: [&str; 2] = ["right", "left"];

/// The variables named by every goal's `vary` (or its defaults), deduplicated. A hand that
/// mirrors the other varies the other's values unless it overrides them itself.
fn collect_params(raw: &Value, resolved: &Value, solved: &Solved, work: &mut Value) -> Result<Vec<Param>, String> {
    let mut patterns: Vec<String> = Vec::new();
    for g in &solved.pose.goals {
        if !g.vary.is_empty() {
            patterns.extend(g.vary.iter().cloned());
            continue;
        }
        if let Some([a, b]) = &g.touch {
            for name in [a, b] {
                let mut parts = name.split('.');
                let (side, part) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
                match part {
                    "thumb" | "index" | "middle" => patterns.push(format!("{side}.{part}.*")),
                    "ring" | "pinky" => {
                        patterns.push(format!("{side}.{part}.*"));
                        patterns.push(format!("{side}.palm_cup"));
                    }
                    _ => patterns.push(format!("{side}.wrist")),
                }
            }
        }
        if let Some(f) = &g.aim {
            let (side, digit) = f.split_once('.').unwrap_or((f, ""));
            patterns.push(format!("{side}.wrist"));
            if digit == "thumb" {
                patterns.push(format!("{side}.thumb.*"));
            } else {
                patterns.push(format!("{side}.{digit}.mcp_flex"));
                patterns.push(format!("{side}.{digit}.mcp_abd"));
            }
        }
    }

    let coupled = solved.pose.version.unwrap_or(0) >= 1;
    let mut out: Vec<Param> = Vec::new();
    for pat in patterns {
        let (side, rest) = pat.split_once('.').ok_or(format!("vary \"{pat}\": write <hand>.<what>, e.g. right.thumb.*"))?;
        if !SIDES.contains(&side) {
            return Err(format!("vary \"{pat}\": the hand must be right or left"));
        }
        if resolved.get("hands").and_then(|h| h.get(side)).is_none() {
            return Err(format!("vary \"{pat}\": there is no {side} hand"));
        }
        if let Some(part) = rest.strip_prefix("place") {
            let part = match part {
                "" => None,
                ".position" => Some(0),
                ".rotation" => Some(1),
                _ => return Err(format!("vary \"{pat}\": use {side}.place, {side}.place.position (move only) or {side}.place.rotation (turn only)")),
            };
            for p in place_params(raw, resolved, side, work, part) {
                if !out.iter().any(|q| q.path == p.path) {
                    out.push(p);
                }
            }
            continue;
        }
        let digit_all = rest.strip_suffix(".*").or(if ["thumb", "index", "middle", "ring", "pinky"].contains(&rest) { Some(rest) } else { None });
        let keys: Vec<&str> = match (rest, digit_all) {
            ("*", _) => PARAMS.iter().map(|p| p.key).collect(),
            (r, _) if r.starts_with("*.") => PARAMS.iter().map(|p| p.key).filter(|k| k.split_once('.').is_some_and(|(_, j)| j == &r[2..])).collect(),
            ("wrist", _) => vec!["twist", "wrist_flex", "wrist_dev"],
            (_, Some(d)) => PARAMS.iter().map(|p| p.key).filter(|k| k.starts_with(&format!("{d}."))).collect(),
            (k, None) => vec![k],
        };
        if keys.is_empty() || keys.iter().any(|k| crate::params::find(k).is_none()) {
            return Err(format!("vary \"{pat}\": use <hand>.<digit>.* , <hand>.<digit>.<joint>, <hand>.*.<joint> (that joint on every digit), <hand>.wrist, <hand>.place, <hand>.* or a parameter such as right.palm_cup (see `handkit explain`)"));
        }
        for key in keys {
            let owner = owner_side(raw, side, Some(key));
            let path: Vec<Value> = std::iter::once("hands").chain(std::iter::once(owner)).chain(key.split('.')).map(|s| json!(s)).collect();
            let current = get(resolved, &path).and_then(Value::as_f64);
            let is_coupled = coupled && key.ends_with("dip_flex") && current.is_none();
            // A coupled dip starts from where the coupling put it.
            let current = match current {
                None if coupled && key.ends_with("dip_flex") => {
                    let pip = std::iter::once("hands").chain(std::iter::once(owner)).chain(key.replace("dip_flex", "pip_flex").split('.').collect::<Vec<_>>()).map(|s| json!(s)).collect::<Vec<_>>();
                    Some(get(resolved, &pip).and_then(Value::as_f64).unwrap_or(0.0) * 2.0 / 3.0)
                }
                c => c,
            };
            if out.iter().any(|q| q.path == path) {
                continue;
            }
            let p = crate::params::find(key).unwrap();
            out.push(Param { label: format!("{owner}.{key}"), path, scale: 10.0, lo: p.min, hi: p.max, start: current.unwrap_or(0.0), coupled: is_coupled });
        }
    }
    Ok(out)
}

/// Which hand's values a mirrored hand's parameter lives in.
fn owner_side<'a>(raw: &'a Value, side: &'a str, key: Option<&str>) -> &'a str {
    let hand = raw.get("hands").and_then(|h| h.get(side));
    let mirrors = hand.and_then(|h| h.get("mirror")).and_then(Value::as_str);
    let Some(other) = mirrors else { return side };
    let own = match key {
        Some(k) => get(hand.unwrap(), &k.split('.').map(|s| json!(s)).collect::<Vec<_>>()).is_some(),
        None => ["place", "elbow", "forearm_rotation_deg"].iter().any(|k| hand.unwrap().get(*k).is_some()),
    };
    if own { side } else if other == "right" { "right" } else { "left" }
}

/// The placement's values: its position (`part` 0), its orientation (1), or both.
fn place_params(raw: &Value, resolved: &Value, side: &str, work: &mut Value, part: Option<usize>) -> Vec<Param> {
    let owner = owner_side(raw, side, None);
    let hand = &resolved["hands"][owner];
    let mut groups: Vec<(&str, Vec<&str>, f64)> = Vec::new();
    if hand.pointer("/place/wrist").is_some_and(|v| !v.is_null()) {
        groups.push(("place", vec!["wrist"], 0.02));
        groups.push(("place", vec!["palm_facing", "fingers_toward"], 0.2));
    } else if hand.get("elbow").is_some_and(|v| !v.is_null()) {
        groups.push(("", vec!["elbow"], 0.02));
        groups.push(("", vec!["forearm_rotation_deg"], 10.0));
    } else {
        let x = if owner == "left" { -0.1 } else { 0.1 };
        let elbow = hand.pointer("/place/elbow").filter(|v| !v.is_null()).cloned().unwrap_or(json!([x, -0.3, 0.0]));
        let rot = hand.pointer("/place/forearm_rotation_deg").filter(|v| !v.is_null()).cloned().unwrap_or(json!([0.0, 0.0, 0.0]));
        work["hands"][owner]["place"] = json!({"elbow": elbow, "forearm_rotation_deg": rot});
        groups.push(("place", vec!["elbow"], 0.02));
        groups.push(("place", vec!["forearm_rotation_deg"], 10.0));
    }
    let mut out = Vec::new();
    for (n, (prefix, keys, scale)) in groups.into_iter().enumerate() {
        if part.is_some_and(|k| k != n) {
            continue;
        }
        for key in keys {
            let mut arr = vec![json!("hands"), json!(owner)];
            if !prefix.is_empty() {
                arr.push(json!(prefix));
            }
            arr.push(json!(key));
            // The placement may come from a preset: write it into the file before varying it.
            if get(work, &arr).is_none_or(|v| !v.is_array()) {
                let from = get(resolved, &arr).filter(|v| v.is_array()).cloned().unwrap_or(json!([0.0, 0.0, 0.0]));
                set_value(work, &arr, from);
                // A hand placed on the screen is then placed by its wrist.
                if let Some(place) = work["hands"][owner].get_mut("place").and_then(Value::as_object_mut).filter(|_| key == "wrist") {
                    place.remove("screen");
                    place.remove("depth");
                }
            }
            for i in 0..3 {
                let mut path = arr.clone();
                path.push(json!(i));
                let start = get(work, &path).or_else(|| get(resolved, &path)).and_then(Value::as_f64).unwrap_or(0.0);
                let label = format!("{owner}.{}{key}[{i}]", if prefix.is_empty() { String::new() } else { format!("{prefix}.") });
                out.push(Param { label, path, scale, lo: f64::NEG_INFINITY, hi: f64::INFINITY, start, coupled: false });
            }
        }
    }
    out
}
