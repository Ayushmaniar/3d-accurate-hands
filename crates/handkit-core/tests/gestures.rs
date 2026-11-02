//! Every library gesture is a finished pose: its goals hold, nothing passes through anything,
//! no joint is past its limit, and `from` can start from it.

use handkit_core::{gestures, pose, solve_value};
use serde_json::json;

#[test]
fn every_gesture_is_solved_and_clean() {
    let list = gestures::list();
    assert!(!list.is_empty());
    let mut bad = Vec::new();
    for (name, desc) in list {
        let v = pose::gesture_value(name).unwrap();
        assert_eq!(v["name"], json!(name), "{name}: \"name\" must match the file name");
        assert!(!desc.is_empty(), "{name}: needs a description");
        assert!(v.get("camera").is_some(), "{name}: needs a camera");
        let q = solve_value(&v).unwrap_or_else(|e| panic!("{name}: {e}")).query().unwrap();
        assert!(!q.goals.is_empty(), "{name}: needs goals for its contacts");
        for g in q.goals.iter().filter(|g| !g.met) {
            bad.push(format!("{name}: {}", g.line()));
        }
        for v in &q.violations {
            bad.push(format!("{name}: {v}"));
        }
        for p in &q.penetrations {
            bad.push(format!("{name}: penetration {} ~ {} {:.1} mm", p.a, p.b, p.gap_mm));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn from_starts_at_a_gesture() {
    let s = solve_value(&json!({"from": "ok"})).unwrap();
    assert!(s.query().unwrap().goals.iter().all(|g| g.met));
    // A hand-level preset renames the gesture's goals to that hand.
    let (v, _) = pose::resolve(json!({"version": 1, "hands": {"left": {"from": "ok"}}})).unwrap();
    assert_eq!(v["goals"][0]["touch"], json!(["left.thumb.pad", "left.index.tip"]));
}
