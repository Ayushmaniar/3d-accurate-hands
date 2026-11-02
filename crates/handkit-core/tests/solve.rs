use handkit_core::solve::{SolveReport, solve_goals};
use serde_json::{Value, json};

fn run(pose: Value) -> (Value, SolveReport) {
    solve_goals(&pose).unwrap_or_else(|e| panic!("{e}"))
}

fn all_met(r: &SolveReport) {
    let lines: Vec<String> = r.goals.iter().map(|g| g.line()).collect();
    assert!(r.goals.iter().all(|g| g.met), "{lines:?}");
    assert_eq!(r.penetrations, 0, "{lines:?}");
}

#[test]
fn pinches_thumb_to_index_from_different_starts() {
    for (thumb, index) in [
        (json!({"cmc_abd": 40, "cmc_rot": 30, "mcp_flex": 20, "ip_flex": 20}), json!({"mcp_flex": 50, "pip_flex": 60})),
        (json!({"cmc_flex": 10, "cmc_abd": 70, "cmc_rot": 20, "mcp_flex": 40, "ip_flex": 40}), json!({"mcp_flex": 50, "pip_flex": 40})),
    ] {
        let (_, r) = run(json!({
            "version": 1,
            "hands": {"right": {"thumb": thumb, "index": index}},
            "goals": [{"touch": ["right.thumb.pad", "right.index.pad"]}],
        }));
        all_met(&r);
    }
}

#[test]
fn writes_the_solution_back_and_rereads_it() {
    let (out, r) = run(json!({
        "version": 1,
        "hands": {"right": {"thumb": {"cmc_abd": 40, "cmc_rot": 30}, "index": {"mcp_flex": 50, "pip_flex": 60}}},
        "goals": [{"touch": ["right.thumb.pad", "right.index.pad"]}],
    }));
    assert!(!r.changes.is_empty());
    let again = handkit_core::solve_value(&out).unwrap().query().unwrap();
    assert!(again.goals.iter().all(|g| g.met));
}

#[test]
fn vary_limits_what_changes() {
    let (out, r) = run(json!({
        "version": 1,
        "hands": {"right": {"thumb": {"cmc_abd": 40, "cmc_rot": 30, "mcp_flex": 20}, "index": {"mcp_flex": 50, "pip_flex": 60}}},
        "goals": [{"touch": ["right.thumb.pad", "right.index.pad"], "vary": ["right.thumb.*"]}],
    }));
    assert!(r.changes.iter().all(|(k, _, _)| k.starts_with("right.thumb.")), "{:?}", r.changes);
    assert_eq!(out["hands"]["right"]["index"], json!({"mcp_flex": 50, "pip_flex": 60}));
}

#[test]
fn moves_mirrored_hands_together() {
    let curl = json!({"mcp_flex": 80, "pip_flex": 90});
    let (out, r) = run(json!({
        "version": 1,
        "hands": {
            "right": {
                "place": {"wrist": [0.08, 0, 0.1], "palm_facing": [-1, 0, 0.3], "fingers_toward": [0, 1, 0]},
                "middle": curl, "ring": curl, "pinky": curl,
            },
            "left": {"mirror": "right"},
        },
        "goals": [{"touch": ["right.index.tip", "left.index.tip"], "vary": ["right.place"]}],
    }));
    all_met(&r);
    // The mirror stays a mirror: only the right hand's placement is written.
    assert_eq!(out["hands"]["left"], json!({"mirror": "right"}));
}

#[test]
fn aims_a_finger() {
    let curl = json!({"mcp_flex": 80, "pip_flex": 90});
    let (_, r) = run(json!({
        "version": 1,
        "hands": {"right": {"middle": curl, "ring": curl, "pinky": curl}},
        "goals": [{"aim": "right.index", "at": [0.4, 0.3, 0.6]}],
    }));
    all_met(&r);
}

#[test]
fn touches_a_segment() {
    let curl = json!({"mcp_flex": 85, "pip_flex": 90});
    let (_, r) = run(json!({
        "version": 1,
        "hands": {"right": {"index": curl, "middle": curl, "ring": curl, "pinky": curl, "thumb": {"cmc_abd": 30, "cmc_flex": 20, "mcp_flex": 40, "ip_flex": 20}}},
        "goals": [{"touch": ["right.thumb.pad", "right.middle.middle"]}],
    }));
    all_met(&r);
}

#[test]
fn explains_bad_goals() {
    let no_goals = solve_goals(&json!({"version": 1, "hands": {"right": {}}})).err().unwrap();
    assert!(no_goals.contains("no goals"), "{no_goals}");
    let typo = solve_goals(&json!({"version": 1, "hands": {"right": {}}, "goals": [{"touch": ["right.thumb.pda", "right.index.pad"]}]})).err().unwrap();
    assert!(typo.contains("unknown site"), "{typo}");
    let vary = solve_goals(&json!({"version": 1, "hands": {"right": {}}, "goals": [{"touch": ["right.thumb.pad", "right.index.pad"], "vary": ["right.thumbs.*"]}]})).err().unwrap();
    assert!(vary.contains("vary"), "{vary}");
}

#[test]
fn leaves_met_goals_and_straight_fingers_alone() {
    let pose = json!({
        "version": 1,
        "hands": {"right": {"thumb": {"cmc_abd": 40, "cmc_rot": 30}, "index": {"mcp_flex": 50, "pip_flex": 60}}},
        "goals": [{"touch": ["right.thumb.pad", "right.index.pad"]}],
    });
    let (solved, _) = run(pose);
    // Solving again changes nothing.
    let (again, r) = run(solved.clone());
    assert!(r.changes.is_empty(), "{:?}", r.changes);
    assert_eq!(again, solved);
    // Values nobody wrote are not written: no wrist, no straight-finger "comfort" bend.
    let right = &solved["hands"]["right"];
    assert!(right.get("twist").is_none() && right.get("palm_cup").is_none(), "{right}");
}

#[test]
fn vary_one_joint_on_every_digit() {
    let (_, r) = run(json!({
        "version": 1,
        "hands": {"right": {}},
        "goals": [{"touch": ["right.index.middle", "right.middle.middle"], "vary": ["right.*.mcp_abd"]}],
    }));
    assert!(r.changes.iter().all(|(k, _, _)| k.ends_with("mcp_abd")), "{:?}", r.changes);
    assert!(r.goals[0].met, "{}", r.goals[0].line());
}

#[test]
fn place_position_moves_without_turning() {
    let curl = json!({"mcp_flex": 80, "pip_flex": 90});
    let (out, r) = run(json!({
        "version": 1,
        "hands": {
            "right": {
                "place": {"wrist": [0.12, 0, 0.1], "palm_facing": [-1, 0, 0.3], "fingers_toward": [0, 1, 0]},
                "middle": curl, "ring": curl, "pinky": curl,
            },
            "left": {"mirror": "right"},
        },
        "goals": [{"touch": ["right.index.tip", "left.index.tip"], "vary": ["right.place.position", "right.index.*"]}],
    }));
    all_met(&r);
    assert_eq!(out["hands"]["right"]["place"]["palm_facing"], json!([-1, 0, 0.3]));
    assert!(r.changes.iter().all(|(k, _, _)| k.contains("place.wrist") || k.contains(".index.")), "{:?}", r.changes);
}
