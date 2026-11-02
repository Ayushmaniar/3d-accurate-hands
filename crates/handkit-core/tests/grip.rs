//! Grips between two people's hands: every style closes onto the other hand without passing
//! through it, and in a clip the gripped hands move together, the follower's arm trailing.

use handkit_core::clip::Clip;
use handkit_core::{grip, solve_value};
use serde_json::json;

fn hands_for(style: &str) -> [&'static str; 2] {
    match style {
        "interlaced" => ["ana.right", "ben.left"],
        _ => ["ana.right", "ben.right"],
    }
}

fn solved(a: &str, b: &str, style: &str) -> handkit_core::Solved {
    let pose = json!({
        "version": 1,
        "hands": { a: {}, b: {} },
        "goals": [{ "grip": [a, b], "style": style }],
    });
    let (v, _) = handkit_core::solve::solve_goals(&pose).unwrap_or_else(|e| panic!("{style}: {e}"));
    solve_value(&v).unwrap()
}

#[test]
fn every_style_closes_without_penetrating() {
    let mut cases: Vec<(&str, [&str; 2])> = grip::NAMES.iter().map(|s| (*s, hands_for(s))).collect();
    // Cupped right with left is the side-by-side hold, placed differently.
    cases.push(("cupped", ["ana.right", "ben.left"]));
    for (style, [a, b]) in cases {
        let q = solved(a, b, style).query().unwrap();
        let g = &q.goals[0];
        assert!(g.met, "{style} {a} ~ {b}: {}", g.line());
        assert!(q.penetrations.is_empty(), "{style}: {:?}", q.penetrations.iter().map(|p| format!("{} ~ {} {:.1}", p.a, p.b, p.gap_mm)).collect::<Vec<_>>());
        assert!(q.violations.is_empty(), "{style}: {:?}", q.violations);
        // The hands touch: some part of each is within a millimetre of the other.
        assert!(g.line().contains("hands 0.0 mm"), "{style}: {}", g.line());
    }
}

#[test]
fn a_grip_writes_editable_values() {
    let pose = json!({
        "version": 1,
        "hands": { "ana.right": {}, "ben.right": {} },
        "goals": [{ "grip": ["ana.right", "ben.right"], "style": "handshake" }],
    });
    let (v, _) = handkit_core::solve::solve_goals(&pose).unwrap();
    // Both hands come back placed, with closed fingers, as plain values.
    for h in ["ana.right", "ben.right"] {
        assert!(v["hands"][h]["place"]["wrist"].is_array(), "{h}: {}", v["hands"][h]);
        assert!(v["hands"][h]["middle"]["pip_flex"].as_f64().unwrap() > 10.0, "{h}: {}", v["hands"][h]);
    }
    // Solving the result again changes nothing: the grip already holds.
    let (again, report) = handkit_core::solve::solve_goals(&v).unwrap();
    assert_eq!(again["hands"], v["hands"]);
    assert_eq!(report.iterations, 0);
}

#[test]
fn grip_needs_a_known_style_and_two_hands() {
    let bad = |goal: serde_json::Value| {
        let pose = json!({ "version": 1, "hands": { "ana.right": {}, "ben.right": {} }, "goals": [goal] });
        handkit_core::solve::solve_goals(&pose).err().unwrap()
    };
    assert!(bad(json!({ "grip": ["ana.right", "ben.right"], "style": "hug" })).contains("handshake"));
    assert!(bad(json!({ "grip": ["ana.right", "ana.right"], "style": "handshake" })).contains("two"));
    assert!(bad(json!({ "grip": ["ana.right", "ben.right"], "style": "interlaced" })).contains("left"));
}

fn demo(name: &str) -> Clip {
    let text = std::fs::read_to_string(format!("{}/../../clips/{name}.json", env!("CARGO_MANIFEST_DIR"))).unwrap();
    Clip::parse(&text).unwrap()
}

fn check(clip: &Clip) -> String {
    let frames: Vec<_> = (0..clip.frames()).map(|i| i as f64 / clip.fps).map(|t| (t, clip.sample(t).unwrap())).collect();
    handkit_core::check::analyze(clip, &frames, 16.0 / 9.0).unwrap().text()
}

#[test]
fn the_grip_demos_pass_check() {
    for name in ["demo_handshake", "demo_holding_hands", "demo_high_five"] {
        let text = check(&demo(name));
        assert!(text.trim_end().ends_with("ok"), "{name}:\n{text}");
        assert!(text.contains("): held"), "{name}:\n{text}");
    }
}

#[test]
fn gripped_hands_move_together_and_the_follower_trails() {
    let clip = demo("demo_handshake");
    let (fps, from, to) = (60.0, 1.6, 3.3);
    let (mut lead, mut elbow, mut rel) = (Vec::new(), Vec::new(), Vec::new());
    for i in 0..=((to - from) * fps) as usize {
        let s = solve_value(&clip.sample(from + i as f64 / fps).unwrap().pose).unwrap();
        let hand = |id: &str| s.hands.iter().find(|h| h.id.name() == id).unwrap();
        let (a, b) = (hand("ana.right"), hand("ben.right"));
        let (fa, fb) = (grip::hand_frame(a), grip::hand_frame(b));
        lead.push(fa.translation.y);
        elbow.push(b.capsules.iter().find(|c| c.part == "forearm").unwrap().a.y);
        rel.push(fa.inverse() * fb);
    }
    // The follower's hand stays where the grip puts it on the leader's, through the pumps.
    for r in &rel {
        assert!((r.translation - rel[0].translation).length() < 0.002, "{:?} vs {:?}", r.translation, rel[0].translation);
        assert!(r.matrix3.x_axis.dot(rel[0].matrix3.x_axis) > 0.999);
    }
    // The leader pumps the hands up and down...
    let span = |v: &[f64]| v.iter().cloned().fold(f64::MIN, f64::max) - v.iter().cloned().fold(f64::MAX, f64::min);
    assert!(span(&lead) > 0.03, "{}", span(&lead));
    // ...and the follower's elbow follows a few frames behind: the best match of its motion to
    // the leader's wrist comes at a delay of 1-8 frames at 30 fps.
    let motion = |v: &[f64]| v.windows(2).map(|w| w[1] - w[0]).collect::<Vec<_>>();
    let (dl, de) = (motion(&lead), motion(&elbow));
    let corr = |lag: usize| dl.iter().zip(&de[lag..]).map(|(a, b)| a * b).sum::<f64>();
    let best = (0..20).max_by(|&a, &b| corr(a).total_cmp(&corr(b))).unwrap();
    assert!((2..=16).contains(&best), "elbow trails by {best} frames at 60 fps");
    // No frame-to-frame jump in the follower's elbow.
    assert!(de.iter().all(|d| d.abs() < 0.01), "{:?}", de.iter().cloned().fold(0.0f64, |m, d| m.max(d.abs())));
}
