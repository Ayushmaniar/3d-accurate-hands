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

#[test]
fn a_pose_near_the_wrist_limits_holds_where_it_was_put() {
    // dev about 33 and flex about -76, within 4 degrees of their limits: the clip must not ease
    // a keyed pose away from them (that turned touching fingertips into each other).
    let hand = r#"{"from": "point", "place": {"wrist": [0, 0.1, 0], "palm_facing": [0, 0, 1], "fingers_toward": [0, 1, 0], "forearm_toward": [0.545, -0.203, -0.814]}}"#;
    let pose = format!(r#"{{"version": 1, "hands": {{"right": {hand}}}}}"#);
    let clip = Clip::parse(&format!(r#"{{"version": 1, "idle": 0, "follow": 0, "keys": [{{"t": 0, "hands": {{"right": {hand}}}}}]}}"#)).unwrap();
    let want = handkit_core::solve(&pose).unwrap();
    let got = handkit_core::solve_value(&clip.sample(1.0).unwrap().pose).unwrap();
    for site in ["right.index.tip", "right.thumb.tip", "right.pinky.tip"] {
        let at = |s: &handkit_core::Solved| s.hands[0].sites.iter().find(|x| x.name == site).unwrap().pos;
        let d = (at(&want) - at(&got)).length();
        assert!(d < 5e-5, "{site} is {:.2} mm from the pose", d * 1000.0);
    }
}

fn bone(s: &handkit_core::Solved, name: &str) -> glam::DVec3 {
    s.hands[0].bones.iter().find(|b| b.name == name).unwrap().origin
}

#[test]
fn screen_places_the_wrist_on_the_frame() {
    // Camera 1 m in front of the origin: the frame's half-height at depth d is d tan(fov / 2).
    let pose = |screen: &str, size: &str| {
        format!(
            r#"{{"version": 1, {size} "camera": {{"position": [0, 0, 1], "target": [0, 0, 0], "fov": 40}},
            "hands": {{"right": {{"place": {{"screen": {screen}, "depth": 0.8, "palm_facing": [0, 0, 1], "fingers_toward": [0, 1, 0]}}}}}}}}"#
        )
    };
    let half = 0.8 * 20f64.to_radians().tan();
    let s = handkit_core::solve(&pose("[0.5, 0.5]", "")).unwrap();
    assert!((bone(&s, "hand") - glam::DVec3::new(0.0, 0.0, 0.2)).length() < 1e-9);
    // Top right corner, default 16:9 and a square frame.
    let s = handkit_core::solve(&pose("[1, 0]", "")).unwrap();
    assert!((bone(&s, "hand") - glam::DVec3::new(half * 16.0 / 9.0, half, 0.2)).length() < 1e-9);
    let s = handkit_core::solve(&pose("[1, 0]", r#""size": [1000, 1000],"#)).unwrap();
    assert!((bone(&s, "hand") - glam::DVec3::new(half, half, 0.2)).length() < 1e-9);
    // No camera: a clear error.
    let e = handkit_core::solve(r#"{"hands": {"right": {"place": {"screen": [0.5, 0.5], "depth": 0.5, "palm_facing": [0, 0, 1], "fingers_toward": [0, 1, 0]}}}}"#).err().unwrap();
    assert!(e.contains("camera"), "{e}");
}

#[test]
fn elbow_screen_puts_the_elbow_on_that_line_of_sight() {
    let s = handkit_core::solve(
        r#"{"version": 1, "camera": {"position": [0, 0, 1], "target": [0, 0, 0], "fov": 40},
        "hands": {"right": {"place": {"screen": [0.6, 0.6], "depth": 0.8, "palm_facing": [0, 0, 1], "fingers_toward": [0, 1, 0], "elbow_screen": [0.75, 0.8]}}}}"#,
    )
    .unwrap();
    let (wrist, elbow) = (bone(&s, "hand"), bone(&s, "forearm"));
    assert!((wrist.distance(elbow) - 0.26).abs() < 1e-9);
    // The elbow projects to x 0.75, y 0.8 of the frame, farther from the camera than the wrist.
    let d = elbow - glam::DVec3::new(0.0, 0.0, 1.0);
    let k = 1.0 / 20f64.to_radians().tan();
    let (x, y) = (d.x / -d.z * k / (16.0 / 9.0), d.y / -d.z * k);
    assert!(((x + 1.0) / 2.0 - 0.75).abs() < 1e-6 && ((1.0 - y) / 2.0 - 0.8).abs() < 1e-6, "{x} {y}");
    assert!(elbow.z < wrist.z);
}

#[test]
fn clips_pass_their_size_and_camera_to_screen_keys() {
    let clip = Clip::parse(
        r#"{"version": 1, "idle": 0, "follow": 0, "size": [1080, 1920], "camera": {"position": [0, 0, 1], "target": [0, 0, 0], "fov": 40}, "keys": [
            {"t": 0, "hands": {"right": {"place": {"screen": [1, 0.5], "depth": 0.8, "palm_facing": [0, 0, 1], "fingers_toward": [0, 1, 0]}}}}]}"#,
    )
    .unwrap();
    let s = handkit_core::solve_value(&clip.sample(0.0).unwrap().pose).unwrap();
    let x = 0.8 * 20f64.to_radians().tan() * 1080.0 / 1920.0;
    assert!((bone(&s, "hand") - glam::DVec3::new(x, 0.0, 0.2)).length() < 1e-6, "{}", bone(&s, "hand"));
}

#[test]
fn hands_keep_their_own_timing() {
    let clip = Clip::parse(
        r#"{"version": 1, "idle": 0, "follow": 0, "keys": [
            {"t": 0, "hands": {"right": {"wrist_flex": 0}, "left": {"wrist_flex": 0}}},
            {"t": 1, "move": 0.2, "hands": {"right": {"wrist_flex": 40}, "left": {"wrist_flex": 40, "move": 1.0, "ease": "linear"}}},
            {"t": 2, "hands": {"right": {"wrist_flex": 0}}},
            {"t": 3, "hands": {"left": {"wrist_flex": 0}}}]}"#,
    )
    .unwrap();
    let at = |side: &str, t: f64| clip.sample(t).unwrap().pose["hands"][side]["wrist_flex"].as_f64().unwrap();
    // The right hand waits for its short move; the left moves the whole second, linearly.
    assert!(at("right", 0.5).abs() < 1e-6);
    assert!((at("left", 0.5) - 20.0).abs() < 1.0, "{}", at("left", 0.5));
    // A key that leaves the left hand out does not hold it: its move back runs from its own
    // last key (t 1) to t 3, through the key at t 2.
    assert!((at("left", 2.0) - 20.0).abs() < 2.0, "{}", at("left", 2.0));
    assert!(at("left", 3.5).abs() < 0.2);
}

/// The right hand sweeps across in front of the left, the fingers overlapping on the way.
const CROSSING: &str = r#"{"version": 1, "idle": 0, "camera": {"position": [0, 0.1, 0.7], "target": [0, 0.1, 0], "fov": 40}, "keys": [
    {"t": 0, "hands": {
        "right": {"from": "relaxed", "place": {"wrist": [0.16, 0.02, 0.012], "palm_facing": [0, 0, 1], "fingers_toward": [-0.3, 1, 0]}},
        "left": {"from": "relaxed", "place": {"wrist": [-0.02, 0.0, -0.012], "palm_facing": [0, 0, 1], "fingers_toward": [0.3, 1, 0]}}}},
    {"t": 1.5, "move": 1.2, "hands": {"right": {"place": {"wrist": [-0.16, 0.02, 0.012], "palm_facing": [0, 0, 1], "fingers_toward": [-0.3, 1, 0]}}}}]}"#;

fn check(clip: &Clip) -> handkit_core::check::CheckReport {
    let frames: Vec<_> = (0..clip.frames()).map(|i| i as f64 / clip.fps).map(|t| (t, clip.sample(t).unwrap())).collect();
    handkit_core::check::analyze(clip, &frames, 16.0 / 9.0).unwrap()
}

#[test]
fn hands_passing_each_other_do_not_pop() {
    // With collisions off, the right hand's fingers pass through the left hand.
    let clip = Clip::parse(&CROSSING.replacen("\"idle\": 0,", "\"idle\": 0, \"collide\": false,", 1)).unwrap();
    let text = check(&clip).text();
    assert!(text.contains("pops: none"), "{text}");
    // The overlap is reported, with the keys around it.
    assert!(text.lines().any(|l| l.starts_with("penetration of the hands, between keys[0] (0.00 s) and keys[1] (1.50 s)")), "{text}");
}

#[test]
fn crossing_hands_slide_past_each_other() {
    let clip = Clip::parse(CROSSING).unwrap();
    let report = check(&clip);
    let text = report.text();
    assert!(text.contains("penetrations: none"), "{text}");
    assert!(text.contains("pops: none"), "{text}");
    // While it crosses, the moving hand (the right) is the one pushed, toward the camera (it is
    // keyed in front of the left). Both hands at rest at the end still overlap a little: then
    // the later-listed hand (the left) takes the push, backward.
    for i in 7..=12 {
        let p = clip.pushes(i as f64 * 0.1);
        assert!(p[0].z > 0.005 && p[1].length() < 0.002, "{p:?} at {}", i as f64 * 0.1);
    }
    let end = clip.pushes(2.5);
    assert!(end[0].length() < 0.002 && end[1].z < 0.0, "{end:?}");
    // Pushes are continuous: no jump between neighbouring samples.
    for i in 0..300 {
        let (a, b) = (clip.pushes(i as f64 * 0.01)[0], clip.pushes((i + 1) as f64 * 0.01)[0]);
        assert!(a.distance(b) < 0.006, "jump at {}", i as f64 * 0.01);
    }
}

#[test]
fn collide_false_on_a_key_lets_hands_overlap() {
    // The right hand is keyed resting inside the left; that key opts out.
    let clip = Clip::parse(
        r#"{"version": 1, "idle": 0, "keys": [
            {"t": 0, "hands": {
                "right": {"place": {"wrist": [0.12, 0.0, 0.0], "palm_facing": [0, 0, 1], "fingers_toward": [0, 1, 0]}},
                "left": {"place": {"wrist": [-0.01, 0.0, 0.0], "palm_facing": [0, 0, 1], "fingers_toward": [0, 1, 0]}}}},
            {"t": 1, "collide": false, "hands": {"right": {"place": {"wrist": [0.0, 0.0, 0.004], "palm_facing": [0, 0, 1], "fingers_toward": [0, 1, 0]}}}},
            {"t": 2, "hands": {"right": {"collide": false}}},
            {"t": 3, "hands": {"right": {"place": {"wrist": [0.12, 0.0, 0.0], "palm_facing": [0, 0, 1], "fingers_toward": [0, 1, 0]}}}}]}"#,
    )
    .unwrap();
    // Off from the move into keys[1] until the move out of keys[2] begins (the right hand's
    // own keys; the left hand's collide stays on, and a pair collides only when both do).
    for t in [0.5, 1.0, 1.5, 1.9] {
        assert!(clip.pushes(t).iter().all(|p| p.length() < 1e-9), "{:?} at {t}", clip.pushes(t));
    }
    // Back on, the overlap is resolved gradually (no pop).
    let text = check(&clip).text();
    assert!(text.contains("penetration of the hands") && text.contains("pops: none"), "{text}");
    assert!(clip.pushes(2.2).iter().any(|p| p.length() > 0.005), "{:?}", clip.pushes(2.2));
    assert!(Clip::parse(r#"{"keys": [{"t": 0, "collide": 1, "hands": {"right": {}}}]}"#).err().unwrap().contains("collide is true or false"));
    assert!(Clip::parse(r#"{"collide": "no", "keys": [{"t": 0, "hands": {"right": {}}}]}"#).is_err());
}

#[test]
fn a_held_goal_between_hands_is_not_pushed_apart() {
    // Steepled fingers and pressed palms: every fingertip (or palm) touches the other hand's,
    // held from key to key while the fingers spread a little.
    for gesture in ["steeple", "prayer"] {
        let clip = Clip::parse(&format!(
            r#"{{"version": 1, "idle": 0, "keys": [{{"t": 0, "from": "{gesture}"}}, {{"t": 1, "from": "{gesture}", "hands": {{"right": {{"ring": {{"mcp_abd": 4}}}}}}}}]}}"#
        ))
        .unwrap();
        let text = check(&clip).text();
        assert!(!text.contains("goal touch") || !text.contains(": off in"), "{gesture}: {text}");
        for i in 0..=10 {
            assert!(clip.pushes(i as f64 * 0.1).iter().all(|p| p.length() < 0.0005), "{gesture}: {:?}", clip.pushes(i as f64 * 0.1));
        }
    }
}

#[test]
fn check_reports_joints_held_at_a_limit() {
    let clip = Clip::parse(
        r#"{"version": 1, "idle": 0, "follow": 0, "keys": [
            {"t": 0, "hands": {"right": {"wrist_flex": 0}}},
            {"t": 0.5, "hands": {"right": {"wrist_flex": 95}}},
            {"t": 1.5, "hands": {"right": {"wrist_flex": 95}}},
            {"t": 2, "hands": {"right": {"wrist_flex": 0}}}]}"#,
    )
    .unwrap();
    let frames: Vec<_> = (0..clip.frames()).map(|i| i as f64 / clip.fps).map(|t| (t, clip.sample(t).unwrap())).collect();
    let report = handkit_core::check::analyze(&clip, &frames, 16.0 / 9.0).unwrap();
    let line = report.lines.iter().find(|l| l.starts_with("at limit: right.wrist_flex at its max 85")).unwrap_or_else(|| panic!("{}", report.text()));
    assert!(line.ends_with("in 0.40-1.60 s"), "{line}");
    assert_eq!(report.problems, 0, "{}", report.text());
}

#[test]
fn an_elbow_in_view_is_fine_when_the_arm_fades_or_is_sleeved() {
    // The whole forearm in view, the elbow at the origin.
    let clip = |look: &str, people: &str| {
        Clip::parse(&format!(
            r#"{{"version": 1, "idle": 0, "duration": 0.5, "camera": {{"position": [0, 0.16, 1.2], "target": [0, 0.16, 0], "fov": 35}}, {look} {people}
                "keys": [{{"t": 0, "hands": {{"ana.right": {{"place": {{"elbow": [0, 0, 0]}}}}}}}}]}}"#
        ))
        .unwrap()
    };
    let report = |c: &Clip| check(c);
    let bare = report(&clip("", ""));
    assert!(bare.lines.iter().any(|l| l.contains("is in view") && l.contains("move it out of frame")), "{}", bare.text());
    assert_eq!(bare.problems, 1, "{}", bare.text());
    for (look, people) in [
        (r#""look": {"arm": "fade"},"#, ""),
        (r#""look": "toon","#, r##""people": {"ana": {"look": {"arm": {"sleeve": "#30406a", "length": 0.1}}}},"##),
    ] {
        let r = report(&clip(look, people));
        assert_eq!(r.problems, 0, "{}", r.text());
        assert!(r.lines.iter().any(|l| l.contains("is in view") && l.contains("fine: its arm fades or is sleeved")), "{}", r.text());
    }
}

#[test]
fn a_site_is_tracked_on_screen() {
    let clip = demo();
    let t = 1.3;
    let frame = clip.sample(t).unwrap();
    let at = frame.sites_on_screen(&["right.index.tip", "right.palm_center", "right.index.distal"]).unwrap();
    assert_eq!(at[0], clip.site_on_screen(t, "right.index.tip").unwrap());
    // Back through the camera: the same point.
    let view = handkit_core::pose::View::of(&frame.pose["camera"]).unwrap();
    for s in &at {
        let p = view.unproject(clip.aspect(), s.screen.unwrap(), s.depth.unwrap());
        assert!(p.distance(s.world) < 1e-9, "{s:?}");
    }
    // A camera looking straight at the fingertip, in a portrait frame: the middle of the frame;
    // a point to its right and above it lands right of and above the middle.
    let tip = at[0].world;
    let mut v: Value = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../clips/demo_basics.json")).unwrap()).unwrap();
    v["camera"] = serde_json::json!({ "position": [tip.x, tip.y, tip.z + 0.5], "target": [tip.x, tip.y, tip.z], "fov": 30 });
    v["size"] = serde_json::json!([1080, 1920]);
    let centred = Clip::from_value(&v).unwrap().site_on_screen(t, "right.index.tip").unwrap();
    let [x, y] = centred.screen.unwrap();
    assert!((x - 0.5).abs() < 1e-5 && (y - 0.5).abs() < 1e-5 && (centred.depth.unwrap() - 0.5).abs() < 1e-5, "{centred:?}");
    let view = handkit_core::pose::View::of(&v["camera"]).unwrap();
    let (s, _) = view.project(1080.0 / 1920.0, tip + glam::DVec3::new(0.01, 0.01, 0.0));
    let [x, y] = s.unwrap();
    // 1 cm at 0.5 m with a 30 degree vertical field of view, in fractions of the frame.
    let k = 1.0 / (15f64.to_radians().tan());
    assert!((x - (0.5 + 0.5 * 0.02 * k / (1080.0 / 1920.0))).abs() < 1e-9 && (y - (0.5 - 0.5 * 0.02 * k)).abs() < 1e-9, "{x} {y}");
    // JSON as the CLI and WASM print it; null on screen without a camera.
    let j = at[0].json();
    assert!(j["screen"].is_array() && j["depth"].is_number() && j["world"].as_array().unwrap().len() == 3, "{j}");
    v.as_object_mut().unwrap().remove("camera");
    let none = Clip::from_value(&v).unwrap().site_on_screen(t, "right.index.tip").unwrap();
    assert!(none.screen.is_none() && none.json()["depth"].is_null());
    assert!(clip.site_on_screen(t, "right.index.tipp").unwrap_err().contains("unknown site"));
}
