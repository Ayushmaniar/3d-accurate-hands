//! Clips: sampling is a pure function of time, keys are reached, motion is continuous, and a
//! goal two keys share holds through the move between them.

use handkit_core::clip::Clip;
use serde_json::Value;

fn demo() -> Clip {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../clips/demo_basics.json")).unwrap();
    Clip::parse(&text).unwrap()
}

fn value(pose: &Value, path: &str) -> f64 {
    path.split('.').fold(&pose["hands"]["right"], |v, k| &v[k]).as_f64().unwrap()
}

#[test]
fn sampling_is_a_pure_function_of_time() {
    let clip = demo();
    let a = clip.sample(2.37).unwrap().pose;
    let _ = clip.sample(4.1).unwrap();
    let b = clip.sample(2.37).unwrap().pose;
    assert_eq!(a, b);
}

#[test]
fn keys_are_reached_and_held() {
    let clip = Clip::parse(
        r#"{"version": 1, "idle": 0, "follow": 0, "keys": [
            {"t": 0, "hands": {"right": {"index": {"mcp_flex": 0}}}},
            {"t": 1, "move": 0.4, "hands": {"right": {"index": {"mcp_flex": 80}}}}]}"#,
    )
    .unwrap();
    let at = |t: f64| value(&clip.sample(t).unwrap().pose, "index.mcp_flex");
    assert!(at(0.0).abs() < 1e-6);
    // The hand holds until the move starts, then arrives near t and settles.
    assert!(at(0.4).abs() < 1e-6);
    assert!((at(1.0) - 80.0).abs() < 4.0, "{}", at(1.0));
    assert!((at(1.5) - 80.0).abs() < 0.2, "{}", at(1.5));
}

#[test]
fn motion_is_continuous() {
    let clip = demo();
    let mut last: Option<Vec<f64>> = None;
    let keys = ["index.mcp_flex", "index.pip_flex", "thumb.cmc_abd", "thumb.ip_flex", "wrist_flex", "pinky.mcp_flex"];
    for i in 0..=(5.2 * 60.0) as usize {
        let pose = clip.sample(i as f64 / 60.0).unwrap().pose;
        let now: Vec<f64> = keys.iter().map(|k| value(&pose, k)).collect();
        if let Some(prev) = &last {
            for (k, (a, b)) in keys.iter().zip(prev.iter().zip(&now)) {
                // At 60 fps no joint jumps more than a fast flick allows.
                assert!((a - b).abs() < 25.0, "{k} jumps {a} -> {b} at {:.3} s", i as f64 / 60.0);
            }
        }
        last = Some(now);
    }
}

#[test]
fn a_shared_goal_holds_through_the_move() {
    let clip = demo();
    // The last move turns the hand while both keys hold the OK pinch.
    for t in [3.7, 3.9, 4.1, 4.3, 4.6] {
        let f = clip.sample(t).unwrap();
        assert!(!f.goals.is_empty(), "no goal held at {t}");
        let mut pose = f.pose.clone();
        pose["goals"] = serde_json::json!(f.goals);
        let solved = handkit_core::solve_value(&pose).unwrap();
        for g in handkit_core::goals::statuses(&solved).unwrap() {
            assert!(g.met, "{} at {t}: {}", g.label, g.value);
        }
    }
}

#[test]
fn the_demo_clip_passes_check() {
    let clip = demo();
    let frames: Vec<_> = (0..clip.frames()).map(|i| i as f64 / clip.fps).map(|t| (t, clip.sample(t).unwrap())).collect();
    let report = handkit_core::check::analyze(&clip, &frames, 16.0 / 9.0).unwrap();
    assert_eq!(report.problems, 0, "{}", report.text());
}

#[test]
fn errors_name_the_key() {
    let e = Clip::parse(r#"{"keys": [{"t": 0, "hands": {"right": {}}}, {"t": 1, "ease": "bouncy"}]}"#).err().unwrap();
    assert!(e.starts_with("keys[1]") && e.contains("ease"), "{e}");
    let e = Clip::parse(r#"{"keys": [{"t": 1, "hands": {"right": {}}}, {"t": 0}]}"#).err().unwrap();
    assert!(e.contains("before"), "{e}");
}

#[test]
fn pass_keys_flow_without_stopping() {
    let clip = Clip::parse(
        r#"{"version": 1, "idle": 0, "follow": 0, "keys": [
            {"t": 0, "hands": {"right": {"wrist_flex": 0}}},
            {"t": 0.5, "pass": true, "hands": {"right": {"wrist_flex": 20}}},
            {"t": 1.0, "pass": true, "hands": {"right": {"wrist_flex": 40}}},
            {"t": 1.5, "hands": {"right": {"wrist_flex": 60}}}]}"#,
    )
    .unwrap();
    let at = |t: f64| value(&clip.sample(t).unwrap().pose, "wrist_flex");
    // Between the first and last key the hand never slows below a third of its average speed.
    for i in 6..=54 {
        let t = i as f64 * 0.025;
        let speed = (at(t + 0.0125) - at(t - 0.0125)) / 0.025;
        assert!(speed > 40.0 / 3.0, "{speed:.1} deg/s at {t:.3} s");
    }
    assert!((at(2.0) - 60.0).abs() < 0.1);
}

#[test]
fn forearm_toward_aims_the_forearm() {
    for side in ["right", "left"] {
        let pose = format!(
            r#"{{"version": 1, "hands": {{"{side}": {{"place": {{"wrist": [0.05, 0.1, 0], "palm_facing": [0.2, -0.3, 1], "fingers_toward": [0.1, 1, 0.3], "forearm_toward": [0.3, -1, 0.25]}}}}}}}}"#
        );
        let s = handkit_core::solve(&pose).unwrap();
        let h = &s.hands[0];
        let forearm = h.bones.iter().find(|b| b.name == "forearm").unwrap();
        let want = glam::DVec3::new(0.3, -1.0, 0.25).normalize();
        assert!((-forearm.along).dot(want) > 0.9999, "{side}: forearm along {:?}", forearm.along);
        let hand = h.bones.iter().find(|b| b.name == "hand").unwrap();
        assert!((hand.origin - glam::DVec3::new(0.05, 0.1, 0.0)).length() < 1e-9);
    }
}
