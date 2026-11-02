//! `handkit check`: motion problems over a clip's frames, before anything is rendered.
//! Held goals that come apart, fingers passing through each other mid-move, joints moving
//! faster than a hand can, pops (a value jumping in one frame between calm frames) and hands
//! leaving the camera's view.

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
    let wrist = DVec3::new(a(e, 0), a(e, 1), a(e, 2)) + rot * DVec3::new(0.0, crate::rig::FOREARM, 0.0);
    Some((v, wrist))
}

/// Projects a point with a pose camera ({position, target, fov}); x and y in [-1, 1] are in view.
fn project(cam: &Value, aspect: f64, p: DVec3) -> Option<(f64, f64)> {
    let a = |k: &str| -> Option<DVec3> {
        let v = cam.get(k)?.as_array()?;
        Some(DVec3::new(v.first()?.as_f64()?, v.get(1)?.as_f64()?, v.get(2)?.as_f64()?))
    };
    let (pos, target) = (a("position")?, a("target")?);
    let fov = cam.get("fov").and_then(Value::as_f64).unwrap_or(35.0).to_radians();
    let f = (target - pos).normalize();
    let r = f.cross(DVec3::Y).normalize();
    let u = r.cross(f);
    let d = p - pos;
    let z = d.dot(f);
    if z <= 0.01 {
        return Some((9.0, 9.0));
    }
    let k = 1.0 / (fov / 2.0).tan();
    Some((d.dot(r) / z * k / aspect, d.dot(u) / z * k))
}

pub fn analyze(clip: &Clip, frames: &[(f64, Frame)], aspect: f64) -> Result<CheckReport, String> {
    let times: Vec<f64> = frames.iter().map(|f| f.0).collect();
    let mut lines = vec![format!("clip: {:.2} s, {} frames at {} fps", clip.duration, frames.len(), clip.fps)];
    let mut problems = 0;

    // Per-frame geometry: goals, penetrations, framing.
    let mut goal_miss: std::collections::BTreeMap<String, (Vec<bool>, f64, f64)> = Default::default();
    let mut pen: std::collections::BTreeMap<String, (Vec<bool>, f64)> = Default::default();
    let mut out_of_view = vec![false; frames.len()];
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
            let e = pen.entry(format!("{} x {}", p.a, p.b)).or_insert((vec![false; frames.len()], 0.0));
            e.0[i] = true;
            e.1 = e.1.min(p.gap_mm);
        }
        if let Some(cam) = pose.get("camera") {
            for (name, p) in &report.tips {
                if let Some((x, y)) = project(cam, aspect, *p) {
                    if x.abs() > 1.0 || y.abs() > 1.0 {
                        out_of_view[i] = true;
                        let _ = name;
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
    if pen.is_empty() {
        lines.push("penetrations: none".into());
    }
    for (pair, (hit, deepest)) in &pen {
        problems += 1;
        lines.push(format!("penetration {pair}: {} (deepest {:.1} mm)", ranges(&times, hit), -deepest));
    }

    // Speeds and pops from the joint values.
    let dt = 1.0 / clip.fps;
    for side in ["right", "left"] {
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
                pops.push(format!("{name} jumps {:.1} mm at {:.2} s", step[i], times[i + 1]));
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
        if out_of_view.iter().any(|x| *x) {
            problems += 1;
            lines.push(format!("framing: a fingertip leaves the camera's view in {}", ranges(&times, &out_of_view)));
        } else {
            lines.push("framing: every fingertip stays in view".into());
        }
    } else {
        lines.push("framing: no camera (add \"camera\" to the clip to check framing)".into());
    }
    lines.push(if problems == 0 { "ok".into() } else { format!("{problems} problem(s)") });
    Ok(CheckReport { lines, problems })
}
