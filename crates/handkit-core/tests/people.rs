//! More than two hands: hands named <person>.right / <person>.left, everywhere a hand is named.

use handkit_core::clip::Clip;
use handkit_core::pose::{HandId, Side};
use serde_json::{Value, json};

fn place(x: f64, z: f64, fingers: [f64; 3]) -> Value {
    json!({ "place": { "wrist": [x, 0.0, z], "palm_facing": [0, 0, 1], "fingers_toward": fingers } })
}

fn four_hands() -> Value {
    json!({
        "version": 1,
        "hands": {
            "right": place(0.10, 0.0, [0.0, 1.0, 0.0]),
            "ana.left": place(-0.30, 0.0, [0.0, 1.0, 0.0]),
            "left": place(-0.10, 0.0, [0.0, 1.0, 0.0]),
            "ana.right": place(0.30, 0.0, [0.0, 1.0, 0.0]),
        },
    })
}

#[test]
fn four_hands_parse_and_solve() {
    let s = handkit_core::solve_value(&four_hands()).unwrap();
    // The default person's hands first, then the others in the order given.
    let names: Vec<String> = s.hands.iter().map(|h| h.name.to_string()).collect();
    assert_eq!(names, ["right", "left", "ana.left", "ana.right"]);
    assert_eq!(s.hands[3].id, HandId::new("ana", Side::Right));
    // Sites and segments carry the hand's name.
    let tip = s.hands[3].sites.iter().find(|x| x.name == "ana.right.index.tip").unwrap();
    assert!((tip.pos.x - 0.33).abs() < 0.02, "{:?}", tip.pos);
    let report = s.query().unwrap();
    assert!(report.penetrations.is_empty());
    assert!(report.tips.iter().any(|(n, _)| n == "ana.left.thumb.tip"));
    assert!(s.fk_text().contains("ana.right.index_pip:"));
    // Bones: the default hands as before, and every hand by its name.
    let bones = s.bones();
    assert!(bones["joints"]["index_pip_R"].is_object() && bones["joints"]["index_pip_L"].is_object());
    assert_eq!(bones["morph"].as_object().unwrap().len(), 2);
    let hands = bones["hands"].as_object().unwrap();
    assert_eq!(hands.keys().collect::<Vec<_>>(), ["right", "left", "ana.left", "ana.right"]);
    assert_ne!(hands["ana.right"]["joints"]["forearm_R"], hands["right"]["joints"]["forearm_R"]);
    assert!(hands["ana.left"]["morph"]["skin_L"].is_array());
    // Geometry names the hand of every capsule.
    let g = s.geometry();
    assert!(g["capsules"].as_array().unwrap().iter().any(|c| c["label"] == "ana.left.forearm" && c["hand"] == "ana.left" && c["side"] == "left"));
}

#[test]
fn prefixed_sites_in_goals_and_checks() {
    let mut pose = four_hands();
    // Ana's right hand points down at the right hand's index tip from above.
    pose["hands"]["ana.right"] = json!({ "place": { "wrist": [0.13, 0.4, 0.0], "palm_facing": [0, 0, 1], "fingers_toward": [0, -1, 0] } });
    pose["goals"] = json!([{ "touch": ["ana.right.index.tip", "right.index.tip"], "vary": ["ana.right.place.position"] }]);
    pose["checks"] = json!([["ana.right.thumb.tip", "left.index.tip"]]);
    let (out, report) = handkit_core::solve::solve_goals(&pose).unwrap();
    assert!(report.goals.iter().all(|g| g.met), "{}", report.text());
    assert!(report.changes.iter().all(|(k, _, _)| k.starts_with("ana.right.place")), "{}", report.text());
    let s = handkit_core::solve_value(&out).unwrap();
    assert_eq!(s.query().unwrap().checks.len(), 1);
    // Aim a person's finger; vary their joints.
    let mut aim = four_hands();
    aim["goals"] = json!([{ "aim": "ana.left.index", "at": [-0.33, 0.4, 0.08] }]);
    let (_, report) = handkit_core::solve::solve_goals(&aim).unwrap();
    assert!(report.goals[0].met, "{}", report.text());
    assert!(report.changes.iter().any(|(k, _, _)| k.starts_with("ana.left.")), "{}", report.text());
    let mut bad = four_hands();
    bad["goals"] = json!([{ "touch": ["bob.right.index.tip", "right.index.tip"] }]);
    assert!(handkit_core::solve::solve_goals(&bad).is_err());
}

#[test]
fn mirror_and_from_take_the_prefix() {
    let pose = json!({
        "version": 1,
        "hands": {
            "ana.right": { "from": "ok", "place": { "wrist": [0.1, 0.0, 0.0], "palm_facing": [0, 0, 1], "fingers_toward": [0, 1, 0] } },
            "ana.left": { "mirror": "ana.right" },
            "ben.left": { "mirror": "ana.right", "place": { "wrist": [-0.3, 0.0, 0.0], "palm_facing": [0, 0, 1], "fingers_toward": [0, 1, 0] } },
        },
    });
    let s = handkit_core::solve_value(&pose).unwrap();
    // The ok gesture's goal comes along, renamed to ana's right hand.
    assert!(s.pose.goals.iter().any(|g| g.touch.as_ref().is_some_and(|t| t[0].starts_with("ana.right."))), "{:?}", s.pose.goals);
    let wrist = |name: &str| s.hands.iter().find(|h| &*h.name == name).unwrap().bones[1].origin;
    let (r, l) = (wrist("ana.right"), wrist("ana.left"));
    assert!((r.x + l.x).abs() < 1e-9 && (r.y - l.y).abs() < 1e-9, "{r:?} {l:?}");
    assert!((wrist("ben.left").x + 0.3).abs() < 1e-9);
    // ana's left copies her right's joint values.
    let thumb = |name: &str| s.hands.iter().find(|h| &*h.name == name).unwrap().sites.iter().find(|x| x.name.ends_with("thumb.tip")).unwrap().pos;
    assert!((thumb("ana.right").x + thumb("ana.left").x).abs() < 1e-9);
    // A mirror names a hand of the other side.
    let bad = json!({ "hands": { "ana.right": {}, "ben.right": { "mirror": "ana.right" } } });
    assert!(handkit_core::solve_value(&bad).err().unwrap().contains("hands.ben.right.mirror must be \"ben.left\""));
}

#[test]
fn hand_names_are_checked() {
    for name in ["Ana.right", "ana.middle", "ana", "right.left", "ana-b.left", "left.right"] {
        let pose = json!({ "hands": { name: {} } });
        let e = handkit_core::solve_value(&pose).err().unwrap_or_else(|| panic!("{name} parsed"));
        assert!(e.contains("unknown hand"), "{name}: {e}");
    }
    for name in ["ana_2.right", "b0b.left", "right", "left"] {
        handkit_core::solve_value(&json!({ "hands": { name: {} } })).unwrap();
    }
}

#[test]
fn people_carry_their_own_look() {
    let mut pose = four_hands();
    pose["look"] = json!({ "style": "toon", "skin": "light" });
    pose["people"] = json!({ "ana": { "look": { "skin": "dark" } } });
    let s = handkit_core::solve_value(&pose).unwrap();
    let ana = handkit_core::pose::person_look(s.pose.look.as_ref(), s.pose.people.as_ref(), "ana").unwrap();
    assert_eq!(ana, json!({ "style": "toon", "skin": "dark" }));
    let own = handkit_core::pose::person_look(s.pose.look.as_ref(), s.pose.people.as_ref(), "").unwrap();
    assert_eq!(own, json!({ "style": "toon", "skin": "light" }));
    for bad in [json!({ "Ana": {} }), json!({ "ana": { "look": { "skin": "teal" } } }), json!({ "ana": { "scale": 2 } })] {
        let mut p = four_hands();
        p["people"] = bad.clone();
        assert!(handkit_core::solve_value(&p).is_err(), "{bad}");
    }
}

#[test]
fn clips_move_four_hands() {
    let mut key0 = four_hands();
    key0.as_object_mut().unwrap().remove("version");
    key0["t"] = json!(0);
    let clip = json!({
        "version": 1, "idle": 0,
        "people": { "ana": { "look": { "skin": "brown" } } },
        "keys": [
            key0,
            // Ana's right hand moves on its own timing; the right hand straightens a finger.
            { "t": 1, "hands": { "ana.right": { "move": 0.5, "place": { "wrist": [0.3, 0.1, 0.0], "palm_facing": [0, 0, 1], "fingers_toward": [0, 1, 0] } } } },
            { "t": 1.5, "hands": { "right": { "index": { "mcp_flex": 60 } } }, "goals": [{ "touch": ["ana.left.thumb.tip", "ana.left.index.tip"] }] },
        ],
    });
    let clip = Clip::from_value(&clip).unwrap();
    assert_eq!(clip.hands().iter().map(|h| h.name()).collect::<Vec<_>>(), ["right", "left", "ana.left", "ana.right"]);
    let f = clip.sample(0.75).unwrap();
    assert_eq!(f.pose["hands"].as_object().unwrap().len(), 4);
    assert_eq!(f.pose["people"]["ana"]["look"]["skin"], "brown");
    // keys[1] names only ana's right hand, with a half-second move: still at 0.5 s.
    let wrist_y = |t: f64| {
        let s = handkit_core::solve_value(&clip.sample(t).unwrap().pose).unwrap();
        s.hands.iter().find(|h| &*h.name == "ana.right").unwrap().bones[1].origin.y
    };
    assert!(wrist_y(0.4).abs() < 1e-3 && (wrist_y(1.2) - 0.1).abs() < 1e-3, "{} {}", wrist_y(0.4), wrist_y(1.2));
    // Check speaks of every hand by its name.
    let frames: Vec<_> = (0..clip.frames()).map(|i| i as f64 / clip.fps).map(|t| (t, clip.sample(t).unwrap())).collect();
    let text = handkit_core::check::analyze(&clip, &frames, 16.0 / 9.0).unwrap().text();
    for name in ["right", "left", "ana.left", "ana.right"] {
        assert!(text.contains(&format!("speed {name}: ok")), "{text}");
    }
    assert!(text.contains("goal touch ana.left.thumb.tip ~ ana.left.index.tip"), "{text}");
    // A hand missing from a later key, or new in one, is an error naming it.
    let missing = json!({ "keys": [{ "t": 0, "hands": { "right": {}, "ana.left": {} } }, { "t": 1, "hands": { "ana.right": {} } }] });
    assert!(Clip::from_value(&missing).err().unwrap().contains("the ana.right hand is not in the first key"));
}

#[test]
fn two_people_stacking_hands_do_not_pass_through() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../clips/demo_four_hands.json")).unwrap();
    let clip = Clip::parse(&text).unwrap();
    assert_eq!(clip.hands().len(), 4);
    let frames: Vec<_> = (0..clip.frames()).map(|i| i as f64 / clip.fps).map(|t| (t, clip.sample(t).unwrap())).collect();
    let report = handkit_core::check::analyze(&clip, &frames, clip.aspect()).unwrap();
    let text = report.text();
    assert!(text.contains("penetrations: none") && text.contains("pops: none"), "{text}");
    assert_eq!(report.problems, 0, "{text}");
}

#[test]
fn old_files_read_as_before() {
    // A two-hand pose: hands, sites and bones as they always were.
    let pose = json!({ "version": 1, "from": "heart_hands" });
    let s = handkit_core::solve_value(&pose).unwrap();
    assert_eq!(s.hands.iter().map(|h| h.name.to_string()).collect::<Vec<_>>(), ["right", "left"]);
    assert!(s.hands.iter().all(|h| h.id.is_default()));
    assert!(s.query().unwrap().goals.iter().all(|g| g.met));
    let bones = s.bones();
    assert_eq!(bones["joints"].as_object().unwrap().len(), 2 * s.hands[0].bones.len());
    // Serializing a pose file keeps hand names as keys.
    let v = serde_json::to_value(&s.pose).unwrap();
    assert!(v["hands"]["right"].is_object() && v["hands"]["left"].is_object());
    // Errors that named a hand still do.
    assert!(handkit_core::solve_value(&json!({ "hands": { "right": { "mirror": "right" } } })).err().unwrap().contains("hands.right.mirror must be \"left\""));
    assert!(handkit_core::solve_value(&json!({ "hands": { "right": { "mirror": "left" }, "left": { "mirror": "right" } } })).err().unwrap().contains("both hands mirror each other"));
}
