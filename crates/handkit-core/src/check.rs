//! `handkit check`: motion problems over a clip's frames, before anything is rendered.
//! Held goals that come apart, fingers passing through each other mid-move, joints moving
//! faster than a hand can, pops (a value jumping in one frame between calm frames) and hands
//! leaving the camera's view; and, as information, joints held at a limit.

use glam::DVec3;
use serde_json::Value;

use crate::clip::{Clip, Frame};
use crate::params::PARAMS;

/// Faster than this, a move reads as a glitch, not a flick (deg/s; m/s for the wrist).
fn speed_limit(key: &str) -> f64 {
    match key {
        k if k.ends_with("_flex") && !k.starts_with("wrist") => 1500.0,
        k if k.ends_with("mcp_abd") || k.ends_with("mcp_roll") => 700.0,
        k if k.starts_with("thumb.") => 1200.0,
        "palm_cup" => 400.0,
        _ => 900.0,
    }
}
const WRIST_SPEED_LIMIT: f64 = 2.5;

pub struct CheckReport {
    pub lines: Vec<String>,
    pub problems: usize,
}

impl CheckReport {
    pub fn text(&self) -> String {
        let mut s = self.lines.join("\n");
        s.push('\n');
        s
    }
}

/// Time ranges where a per-frame condition holds, as "1.20-1.43 s".
fn ranges(times: &[f64], hit: &[bool]) -> String {
    let mut out = Vec::new();
    let mut i = 0;
    while i < hit.len() {
        if !hit[i] {
            i += 1;
            continue;
        }
        let a = i;
        while i + 1 < hit.len() && hit[i + 1] {
            i += 1;
        }
        out.push(if a == i { format!("{:.2} s", times[a]) } else { format!("{:.2}-{:.2} s", times[a], times[i]) });
        i += 1;
    }
    if out.len() > 6 {
        let n = out.len();
        out.truncate(6);
        out.push(format!("and {} more", n - 6));
    }
    out.join(", ")
}

fn values(pose: &Value, side: &str) -> Option<(Vec<f64>, DVec3)> {
    let h = pose.get("hands")?.get(side)?;
    let v = PARAMS
        .iter()
        .map(|p| p.key.split('.').try_fold(h, |v, k| v.get(k)).and_then(Value::as_f64).unwrap_or(0.0))
        .collect();
    let e = h.pointer("/place/elbow")?;
    let r = h.pointer("/place/forearm_rotation_deg")?;
    let a = |v: &Value, i: usize| v[i].as_f64().unwrap_or(0.0);
    let rot = glam::DQuat::from_euler(glam::EulerRot::XYZ, a(r, 0).to_radians(), a(r, 1).to_radians(), a(r, 2).to_radians());
    let k = crate::pose::split_hand(side).and_then(|(id, _)| crate::pose::person_scale(pose.get("people"), &id.person)).unwrap_or(1.0);
    let wrist = DVec3::new(a(e, 0), a(e, 1), a(e, 2)) + rot * DVec3::new(0.0, crate::rig::FOREARM * k, 0.0);
    Some((v, wrist))
}

/// Projects a point with a pose camera ({position, target, fov}); x and y in [-1, 1] are in view.
fn project(cam: &Value, aspect: f64, p: DVec3) -> Option<(f64, f64)> {
    let v = crate::pose::View::of(cam)?;
    let d = p - v.pos;
    let z = d.dot(v.f);
    if z <= 0.01 {
        return Some((9.0, 9.0));
    }
    Some((d.dot(v.r) / z * v.k / aspect, d.dot(v.u) / z * v.k))
}

/// Frames, deepest overlap (mm), its time and pair, and every pair of parts.
type HandsOverlap = (Vec<bool>, f64, f64, String, std::collections::BTreeSet<String>);

pub fn analyze(clip: &Clip, frames: &[(f64, Frame)], aspect: f64) -> Result<CheckReport, String> {
    let times: Vec<f64> = frames.iter().map(|f| f.0).collect();
    // Every hand, in the clip's order.
    let names: Vec<String> = clip.hands().iter().map(|h| h.name()).collect();
    let mut lines = vec![format!("clip: {:.2} s, {} frames at {} fps", clip.duration, frames.len(), clip.fps)];
    let mut problems = 0;

    // Per-frame geometry: goals, penetrations, framing.
    let mut goal_miss: std::collections::BTreeMap<String, (Vec<bool>, f64, f64)> = Default::default();
    let mut pen: std::collections::BTreeMap<String, (Vec<bool>, f64, f64)> = Default::default();
    // The two hands overlapping, per move (by the keys either side): the frames, the deepest
    // overlap, when and between which parts, and every pair of parts.
    let mut cross: std::collections::BTreeMap<(usize, String), HandsOverlap> = Default::default();
    let mut out_of_view: std::collections::BTreeMap<String, (Vec<bool>, f64)> = Default::default();
    // An elbow in view: a problem for a bare arm, information for one that fades or is sleeved.
    let mut arm_end = vec![false; frames.len()];
    let mut arm_end_hidden = vec![false; frames.len()];
    let base_look = clip.look.as_ref().and_then(|l| crate::look::Look::from_value(l).ok()).unwrap_or_default();
    let hides_end: Vec<bool> = clip.hands().iter().map(|id| base_look.for_person(clip.people.as_ref(), &id.person).arm.hides_end()).collect();
    let mut site_tracks: std::collections::BTreeMap<String, Vec<DVec3>> = Default::default();
    for (i, (_, f)) in frames.iter().enumerate() {
        let mut pose = f.pose.clone();
        pose["goals"] = serde_json::json!(f.goals);
        let solved = crate::solve_value(&pose)?;
        let report = crate::query::analyze(&solved.pose, &solved.hands)?;
        for g in crate::goals::statuses(&solved)? {
            let e = goal_miss.entry(g.label.clone()).or_insert((vec![false; frames.len()], 0.0, 0.0));
            if !g.met {
                e.0[i] = true;
                if g.value.abs() > e.1.abs() {
                    e.1 = g.value;
                    e.2 = frames[i].0;
                }
            }
        }
        for p in &report.penetrations {
            let pair = format!("{} x {}", p.a, p.b);
            if crate::pose::hand_of(&p.a) != crate::pose::hand_of(&p.b) {
                let k = clip.keys_around(frames[i].0);
                let e = cross.entry((clip.key_before(frames[i].0), k)).or_insert((vec![false; frames.len()], 0.0, 0.0, String::new(), Default::default()));
                e.0[i] = true;
                if p.gap_mm < e.1 {
                    (e.1, e.2, e.3) = (p.gap_mm, frames[i].0, pair.clone());
                }
                e.4.insert(pair);
                continue;
            }
            let e = pen.entry(pair).or_insert((vec![false; frames.len()], 0.0, 0.0));
            e.0[i] = true;
            if p.gap_mm < e.1 {
                (e.1, e.2) = (p.gap_mm, frames[i].0);
            }
        }
        if let Some(cam) = pose.get("camera") {
            for (name, p) in &report.tips {
                let Some((x, y)) = project(cam, aspect, *p) else { continue };
                for (over, edge) in [(x - 1.0, "right"), (-x - 1.0, "left"), (y - 1.0, "top"), (-y - 1.0, "bottom")] {
                    if over > 0.0 {
                        let e = out_of_view.entry(format!("{name} past the {edge} edge")).or_insert((vec![false; frames.len()], 0.0));
                        e.0[i] = true;
                        // In percent of the half-frame, so 10 = a tenth of the way to the centre.
                        e.1 = e.1.max(over * 100.0);
                    }
                }
            }
        }
        if let Some(cam) = pose.get("camera") {
            // The rig ends at the elbow: a bare forearm's rounded end must stay out of the frame.
            for (side, hidden) in names.iter().zip(&hides_end) {
                let Some(e) = pose.get("hands").and_then(|h| h.get(side)).and_then(|h| h.pointer("/place/elbow")).and_then(Value::as_array) else { continue };
                let e = DVec3::new(e[0].as_f64().unwrap_or(0.0), e[1].as_f64().unwrap_or(0.0), e[2].as_f64().unwrap_or(0.0));
                if let Some((x, y)) = project(cam, aspect, e) {
                    if x.abs() < 1.0 && y.abs() < 1.0 {
                        if *hidden {
                            arm_end_hidden[i] = true;
                        } else {
                            arm_end[i] = true;
                        }
                    }
                }
            }
        }
        for (name, p) in &report.tips {
            site_tracks.entry(name.clone()).or_default().push(*p);
        }
    }
    if goal_miss.is_empty() {
        lines.push("goals: none held".into());
    }
    for (label, (miss, worst, at)) in &goal_miss {
        if miss.iter().any(|x| *x) {
            problems += 1;
            lines.push(format!("goal {label}: off in {} (worst {worst:.1} at {at:.2} s)", ranges(&times, miss)));
        } else {
            lines.push(format!("goal {label}: held"));
        }
    }
    if pen.is_empty() && cross.is_empty() {
        lines.push("penetrations: none".into());
    }
    for (pair, (hit, deepest, at)) in &pen {
        problems += 1;
        lines.push(format!("penetration {pair}: {} (deepest {:.1} mm at {at:.2} s, {})", ranges(&times, hit), -deepest, clip.keys_around(*at)));
    }
    for ((_, keys), (hit, deepest, at, pair, pairs)) in &cross {
        problems += 1;
        lines.push(format!(
            "penetration of the hands, {keys}: {} ({} {}; deepest {:.1} mm, {pair} at {at:.2} s)",
            ranges(&times, hit),
            pairs.len(),
            if pairs.len() == 1 { "pair of parts" } else { "pairs of parts" },
            -deepest
        ));
    }

    // Speeds and pops from the joint values.
    let dt = 1.0 / clip.fps;
    for side in &names {
        let tracks: Vec<(Vec<f64>, DVec3)> = frames.iter().filter_map(|f| values(&f.1.pose, side)).collect();
        if tracks.len() != frames.len() || tracks.len() < 3 {
            continue;
        }
        let mut fast = Vec::new();
        for (j, p) in PARAMS.iter().enumerate() {
            let (mut peak, mut at) = (0.0_f64, 0.0);
            for i in 1..tracks.len() {
                let s = (tracks[i].0[j] - tracks[i - 1].0[j]).abs() / dt;
                if s > peak {
                    peak = s;
                    at = times[i];
                }
            }
            if peak > speed_limit(p.key) {
                fast.push(format!("{side}.{} {peak:.0} deg/s at {at:.2} s (limit {:.0})", p.key, speed_limit(p.key)));
            }
        }
        let (mut wpeak, mut wat) = (0.0_f64, 0.0);
        for i in 1..tracks.len() {
            let s = (tracks[i].1 - tracks[i - 1].1).length() / dt;
            if s > wpeak {
                wpeak = s;
                wat = times[i];
            }
        }
        if wpeak > WRIST_SPEED_LIMIT {
            fast.push(format!("{side} wrist {wpeak:.1} m/s at {wat:.2} s (limit {WRIST_SPEED_LIMIT})"));
        }
        if fast.is_empty() {
            lines.push(format!("speed {side}: ok (wrist peak {wpeak:.2} m/s at {wat:.2} s)"));
        } else {
            problems += fast.len();
            for f in fast {
                lines.push(format!("too fast: {f}"));
            }
        }
    }
    // Joints held at a limit (information: the pose asks for more than the joint allows, and
    // the clip eases it to the limit).
    for side in &names {
        let tracks: Vec<Vec<f64>> = frames.iter().filter_map(|f| values(&f.1.pose, side).map(|v| v.0)).collect();
        if tracks.len() != frames.len() {
            continue;
        }
        for (j, p) in PARAMS.iter().enumerate() {
            for (limit, name) in [(p.min, "min"), (p.max, "max")] {
                let near: Vec<bool> = tracks.iter().map(|v| (v[j] - limit).abs() <= 1.0).collect();
                // Only runs longer than 0.3 s.
                let mut long = vec![false; near.len()];
                let mut i = 0;
                while i < near.len() {
                    let a = i;
                    while i < near.len() && near[i] {
                        i += 1;
                    }
                    if i > a && (i - a) as f64 * dt > 0.3 {
                        long[a..i].fill(true);
                    }
                    i = i.max(a + 1);
                }
                if long.iter().any(|x| *x) {
                    lines.push(format!("at limit: {side}.{} at its {name} {limit} in {}", p.key, ranges(&times, &long)));
                }
            }
        }
    }
    // A pop: one frame's step is much bigger than both neighbours' and visibly large.
    let mut pops = Vec::new();
    for (name, track) in &site_tracks {
        if track.len() != frames.len() {
            continue;
        }
        let step: Vec<f64> = track.windows(2).map(|w| (w[1] - w[0]).length() * 1000.0).collect();
        for i in 1..step.len().saturating_sub(1) {
            let calm = step[i - 1].max(step[i + 1]);
            if step[i] > 4.0 && step[i] > 3.0 * calm {
                pops.push(format!("{name} jumps {:.1} mm at {:.2} s ({})", step[i], times[i + 1], clip.keys_around(times[i + 1])));
            }
        }
    }
    if pops.is_empty() {
        lines.push("pops: none".into());
    } else {
        problems += pops.len();
        let n = pops.len();
        lines.extend(pops.into_iter().take(8).map(|p| format!("pop: {p}")));
        if n > 8 {
            lines.push(format!("pop: and {} more", n - 8));
        }
    }
    if frames.iter().any(|f| f.1.pose.get("camera").is_some()) {
        if !out_of_view.is_empty() {
            problems += 1;
            for (what, (hit, worst)) in &out_of_view {
                lines.push(format!("framing: {what} in {} (at most {worst:.0}% of the half-frame out)", ranges(&times, hit)));
            }
        } else {
            lines.push("framing: every fingertip stays in view".into());
        }
        if arm_end.iter().any(|x| *x) {
            problems += 1;
            lines.push(format!("framing: an elbow (where the forearm ends) is in view in {}; move it out of frame, or give the look an \"arm\" (fade or a sleeve)", ranges(&times, &arm_end)));
        }
        if arm_end_hidden.iter().any(|x| *x) {
            lines.push(format!("framing: an elbow is in view in {} (fine: its arm fades or is sleeved)", ranges(&times, &arm_end_hidden)));
        }
    } else {
        lines.push("framing: no camera (add \"camera\" to the clip to check framing)".into());
    }
    lines.push(if problems == 0 { "ok".into() } else { format!("{problems} problem(s)") });
    Ok(CheckReport { lines, problems })
}
