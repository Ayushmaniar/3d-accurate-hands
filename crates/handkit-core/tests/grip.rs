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

fn hand<'a>(s: &'a handkit_core::Solved, id: &str) -> &'a handkit_core::rig::HandGeom {
    s.hands.iter().find(|h| h.id.name() == id).unwrap()
}

fn value(s: &handkit_core::Solved, id: &str, key: &str) -> f64 {
    let hp = s.pose.hands.0.iter().find(|(h, _)| h.name() == id).unwrap().1.clone();
    let v = serde_json::to_value(hp).unwrap();
    key.split('.').fold(&v, |v, k| &v[k]).as_f64().unwrap_or(0.0)
}

/// Where a point is in a hand's frame: z > 0 off its palm, z < 0 behind its back.
fn in_frame(h: &handkit_core::rig::HandGeom, p: glam::DVec3) -> glam::DVec3 {
    grip::hand_frame(h).inverse().transform_point3(p)
}

fn tip(h: &handkit_core::rig::HandGeom, digit: &str) -> glam::DVec3 {
    h.sites.iter().find(|s| s.name == format!("{}.{digit}.tip", h.name)).unwrap().pos
}

const FOUR: [&str; 4] = ["index", "middle", "ring", "pinky"];

#[test]
fn interlaced_fingers_alternate_and_wrap_the_backs() {
    let s = solved("ana.right", "ben.left", "interlaced");
    let (a, b) = (hand(&s, "ana.right"), hand(&s, "ben.left"));
    // Every fingertip ends behind the other hand's palm, over its back.
    for (own, other) in [(a, b), (b, a)] {
        for d in FOUR {
            let z = in_frame(other, tip(own, d)).z;
            assert!(z < 0.0, "{}.{d} tip is {:.1} mm off {}'s palm, not over its back", own.name, z * 1000.0, other.name);
        }
    }
    // Across the hands (from the thumbs), the proximal segments alternate: ben's index, ana's
    // index, ben's middle, ana's middle, ...
    let mut order: Vec<(f64, String)> = [a, b]
        .iter()
        .flat_map(|h| h.capsules.iter().filter(|c| c.part != "thumb" && c.seg == handkit_core::rig::Seg::Proximal).map(move |c| (in_frame(a, (c.a + c.b) / 2.0).x, h.name.to_string())))
        .collect();
    order.sort_by(|x, y| y.0.total_cmp(&x.0));
    let who: Vec<&str> = order.iter().map(|(_, n)| if n.starts_with("ana") { "a" } else { "b" }).collect();
    assert_eq!(who.concat(), "babababa", "{order:?}");
    // Both thumbs lie over the other hand's back.
    for (own, other) in [(a, b), (b, a)] {
        assert!(in_frame(other, tip(own, "thumb")).z < 0.0, "{} thumb", own.name);
    }
}

#[test]
fn cupped_side_by_side_wraps_the_inner_hand() {
    let s = solved("ana.right", "ben.left", "cupped");
    let (a, b) = (hand(&s, "ana.right"), hand(&s, "ben.left"));
    // The outer (follower) hand's fingers curl round the inner hand's edge onto its back...
    let behind = FOUR.iter().filter(|d| in_frame(a, tip(b, d)).z < 0.0).count();
    assert!(behind >= 3, "{behind} of ben's fingertips over ana's back");
    // ...and its thumb closes over the back of the inner hand too, not left open.
    assert!(in_frame(a, tip(b, "thumb")).z < 0.0);
    assert!(value(&s, "ben.left", "thumb.mcp_flex") + value(&s, "ben.left", "thumb.ip_flex") > 25.0);
}

#[test]
fn pinky_promise_hooks_both_pinkies() {
    let s = solved("ana.right", "ben.right", "pinky");
    for h in ["ana.right", "ben.right"] {
        // Each pinky is bent into a hook, the other fingers curled into a fist.
        assert!(value(&s, h, "pinky.pip_flex") > 60.0, "{h} pinky pip {}", value(&s, h, "pinky.pip_flex"));
        for d in ["index", "middle", "ring"] {
            assert!(value(&s, h, &format!("{d}.mcp_flex")) > 60.0 && value(&s, h, &format!("{d}.pip_flex")) > 80.0, "{h} {d}");
        }
    }
    // The pinkies cross: each one's tip ends on the far side of the other's middle segment.
    let g = &s.query().unwrap().goals[0];
    assert!(g.line().contains("pinky -0.0 on") || g.line().contains("pinky 0.0 on"), "{}", g.line());
}

#[test]
fn a_child_and_an_adult_shake_hands() {
    let pose = json!({
        "version": 1,
        "hands": { "ana.right": {}, "mia.right": {} },
        "people": { "mia": { "scale": 0.75 } },
        "goals": [{ "grip": ["ana.right", "mia.right"], "style": "handshake" }],
    });
    let (v, _) = handkit_core::solve::solve_goals(&pose).unwrap();
    let s = solve_value(&v).unwrap();
    let q = s.query().unwrap();
    assert!(q.goals[0].met, "{}", q.goals[0].line());
    assert!(q.penetrations.is_empty() && q.violations.is_empty());
    // The child's hand is three quarters the size: segments, thickness and the forearm.
    let len = |h: &str, part: &str| {
        let c = hand(&s, h).capsules.iter().find(|c| c.label() == format!("{h}.{part}")).unwrap();
        (c.a.distance(c.b), c.r)
    };
    for part in ["middle.proximal", "thumb.distal", "forearm"] {
        let ((la, ra), (lm, rm)) = (len("ana.right", part), len("mia.right", part));
        assert!((lm / la - 0.75).abs() < 1e-6 && (rm / ra - 0.75).abs() < 1e-6, "{part}");
    }
    // A mesh adapter gets the size on the root joint, the rest as the default hand's.
    let bones = s.bones();
    assert_eq!(bones["hands"]["mia.right"]["joints"]["forearm_R"]["s"], json!(0.75));
    assert!(bones["hands"]["ana.right"]["joints"]["forearm_R"].get("s").is_none());
    assert_eq!(bones["hands"]["mia.right"]["joints"]["index_pip_R"]["t"], bones["hands"]["ana.right"]["joints"]["index_pip_R"]["t"]);
    // A scale out of range is refused.
    let mut bad = pose.clone();
    bad["people"]["mia"]["scale"] = json!(3.0);
    assert!(handkit_core::solve::solve_goals(&bad).err().unwrap().contains("scale"));
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
fn a_childs_hand_is_solved_and_played_at_its_size() {
    // The demo handshake with Ben as a child: each key is solved at the size it plays at, and
    // the forearm between wrist and elbow is the child's, so the grip holds without overlap.
    let path = format!("{}/../../clips/demo_handshake.json", env!("CARGO_MANIFEST_DIR"));
    let mut v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    v["people"]["ben"]["scale"] = json!(0.75);
    let clip = Clip::parse(&v.to_string()).unwrap();
    let text = check(&clip);
    assert!(!text.contains("penetration of the hands"), "{text}");
    let s = solve_value(&clip.sample(2.0).unwrap().pose).unwrap();
    let ben = s.hands.iter().find(|h| h.id.name() == "ben.right").unwrap();
    let arm = ben.capsules.iter().find(|c| c.part == "forearm").unwrap();
    assert!(((grip::hand_frame(ben).translation - arm.a).length() - 0.26 * 0.75).abs() < 1e-6);
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

#[test]
fn a_chain_of_three_people_keeps_both_holds() {
    // Ben holds Ana with one hand and a child with the other. Each hold keeps its follower
    // where the first key's grip put it, through every swing, even the child's, which is not
    // quite met (an adult's hand can't wrap a child's fully).
    let clip = demo("demo_three_in_a_row");
    let rel = |t: f64, lead: &str, follow: &str| {
        let s = solve_value(&clip.sample(t).unwrap().pose).unwrap();
        let h = |id: &str| grip::hand_frame(s.hands.iter().find(|h| h.id.name() == id).unwrap());
        h(lead).inverse() * h(follow)
    };
    for (lead, follow) in [("ana.right", "ben.left"), ("ben.right", "cal.left")] {
        let first = rel(0.6, lead, follow);
        for t in [0.9, 1.4, 2.5, 4.0] {
            let r = rel(t, lead, follow);
            assert!((r.translation - first.translation).length() < 0.001, "{follow} at {t}: {:?} vs {:?}", r.translation, first.translation);
            assert!(r.matrix3.y_axis.dot(first.matrix3.y_axis) > 0.9995, "{follow} at {t}");
        }
    }
    let text = check(&clip);
    assert!(!text.contains("pop:"), "{text}");
}
