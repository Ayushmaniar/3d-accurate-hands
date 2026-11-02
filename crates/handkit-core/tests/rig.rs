use glam::DVec3;
use handkit_core::solve;

fn site(s: &handkit_core::Solved, name: &str) -> DVec3 {
    s.hands.iter().flat_map(|h| &h.sites).find(|x| x.name == name).unwrap_or_else(|| panic!("no site {name}")).pos
}

fn close(a: DVec3, b: DVec3, tol: f64) -> bool {
    a.distance(b) <= tol
}

#[test]
fn matches_the_experiment_rig() {
    // Every pose the agents wrote in the joint-angle experiment lands where the experiment's
    // renderer put it (its reports round to 1 mm).
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../experiments/joint-angles/runs");
    let mut checked = 0;
    for run in ["blind-sonnet", "blind-opus", "loop-opus"] {
        for t in 1..=7 {
            let pose = std::fs::read_to_string(format!("{root}/{run}/T{t}.json")).unwrap();
            let report: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(format!("{root}/{run}/T{t}.report.json")).unwrap()).unwrap();
            let solved = solve(&pose).unwrap();
            for (name, p) in report["fingertip_world_m"].as_object().unwrap() {
                let want = DVec3::new(p[0].as_f64().unwrap(), p[1].as_f64().unwrap(), p[2].as_f64().unwrap());
                let got = solved.hands.iter().flat_map(|h| &h.capsules).find(|c| c.label() == format!("{name}.distal")).unwrap().b;
                assert!((got - want).abs().max_element() <= 0.0005 + 1e-9, "{run}/T{t} {name}: {got} vs {want}");
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 7 * 3 * 5 + 2 * 3 * 5);
}

#[test]
fn hand_placement_matches_forearm_placement_at_rest() {
    let a = solve(r#"{"version":1,"hands":{"right":{"place":{"wrist":[0.1,0,0],"palm_facing":[0,0,1],"fingers_toward":[0,1,0]}}}}"#).unwrap();
    let b = solve(r#"{"version":1,"hands":{"right":{"place":{"elbow":[0.1,-0.26,0]}}}}"#).unwrap();
    for (x, y) in a.hands[0].sites.iter().zip(&b.hands[0].sites) {
        assert!(close(x.pos, y.pos, 1e-9), "{}", x.name);
    }
}

#[test]
fn hand_placement_holds_with_wrist_angles() {
    // The hand frame is placed exactly, whatever twist and wrist angles do to the forearm.
    let s = solve(
        r#"{"version":1,"hands":{"left":{"place":{"wrist":[0,0.2,0.1],"palm_facing":[0,-1,0],"fingers_toward":[1,0,0]},
            "twist":40,"wrist_flex":30,"wrist_dev":-10}}}"#,
    )
    .unwrap();
    let h = &s.hands[0];
    let hand = h.bones.iter().find(|b| b.name == "hand").unwrap();
    assert!(close(hand.origin, DVec3::new(0.0, 0.2, 0.1), 1e-9));
    assert!(close(hand.along, DVec3::X, 1e-9));
    assert!(close(hand.palmar, DVec3::NEG_Y, 1e-9));
    // A right hand's thumb is on the fingers x palm side (-Z here); a left hand's is opposite.
    let thumb = site(&s, "left.thumb.tip");
    let pinky = site(&s, "left.pinky.tip");
    assert!(thumb.z > pinky.z, "left thumb should be toward +Z here: {thumb} vs {pinky}");
}

#[test]
fn mirrored_hand_is_the_reflection() {
    let s = solve(
        r#"{"version":1,"hands":{"right":{"place":{"wrist":[0.07,0.01,0.02],"palm_facing":[-1,0,0.3],"fingers_toward":[0,1,-0.2]},
            "twist":15,"index":{"mcp_flex":40},"thumb":{"cmc_abd":30,"cmc_rot":20}},"left":{"mirror":"right"}}}"#,
    )
    .unwrap();
    let (r, l) = (&s.hands[0], &s.hands[1]);
    for (a, b) in r.sites.iter().zip(&l.sites) {
        let m = DVec3::new(-a.pos.x, a.pos.y, a.pos.z);
        assert!(close(m, b.pos, 1e-9), "{} vs {}", a.name, b.name);
    }
}

#[test]
fn mirror_overrides_apply() {
    let s = solve(r#"{"version":1,"hands":{"right":{"index":{"mcp_flex":80}},"left":{"mirror":"right","index":{"mcp_flex":0}}}}"#).unwrap();
    let l = &s.hands[1];
    let idx = l.bones.iter().find(|b| b.name == "index_mcp").unwrap();
    assert!(idx.along.y > 0.99, "left index should be straight: {}", idx.along);
}

#[test]
fn flexion_curls_toward_the_palm() {
    let s = solve(r#"{"version":1,"hands":{"right":{"place":{"wrist":[0,0,0],"palm_facing":[0,0,1],"fingers_toward":[0,1,0]},"index":{"mcp_flex":90}}}}"#).unwrap();
    let pip = s.hands[0].bones.iter().find(|b| b.name == "index_pip").unwrap();
    assert!(pip.along.z > 0.99, "{}", pip.along);
}

#[test]
fn dip_follows_pip_only_in_version_1() {
    let v1 = solve(r#"{"version":1,"hands":{"right":{"middle":{"pip_flex":60}}}}"#).unwrap();
    assert_eq!(v1.hands[0].coupled.len(), 1);
    let dip = v1.hands[0].bones.iter().find(|b| b.name == "middle_dip").unwrap();
    let pip = v1.hands[0].bones.iter().find(|b| b.name == "middle_pip").unwrap();
    assert!((pip.along.angle_between(dip.along).to_degrees() - 40.0).abs() < 1e-6);
    let legacy = solve(r#"{"hands":{"right":{"middle":{"pip_flex":60}}}}"#).unwrap();
    assert!(legacy.hands[0].coupled.is_empty());
}

#[test]
fn reports_limits_contacts_and_typos() {
    let s = solve(r#"{"version":1,"hands":{"right":{"index":{"pip_flex":130}}}}"#).unwrap();
    assert_eq!(s.query().unwrap().violations, vec!["right.index.pip_flex=130 outside [-5, 110]"]);

    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../experiments/joint-angles/runs/loop-opus/T6.json");
    let ok_sign = solve(&std::fs::read_to_string(root).unwrap()).unwrap().query().unwrap();
    assert!(ok_sign.contacts.iter().any(|p| p.a == "right.index.distal" && p.b == "right.thumb.distal"));

    let err = solve(r#"{"hands":{"right":{"index":{"pip_flx":10}}}}"#).err().unwrap();
    assert!(err.contains("pip_flx"), "{err}");
    let err = solve(r#"{"version":1,"hands":{},"checks":[["right.thumb.pad","right.index.pad"]]}"#).unwrap().query().err().unwrap();
    assert!(err.contains("unknown site"), "{err}");
}

#[test]
fn probes_measure_aim() {
    // A straight index finger pointing +Y aims exactly at a point straight ahead of it.
    let s = solve(
        r#"{"version":1,"hands":{"right":{"place":{"wrist":[0,0,0],"palm_facing":[0,0,1],"fingers_toward":[0,1,0]},"index":{"mcp_abd":-4}}},
            "probes":{"p":[0.030,1.0,0]}}"#,
    )
    .unwrap();
    let r = s.query().unwrap();
    let hit = r.probes.iter().find(|p| p.finger == "right.index").unwrap();
    assert!(hit.angle_deg < 0.05 && hit.miss_mm < 0.5, "{} deg {} mm", hit.angle_deg, hit.miss_mm);
}

#[test]
fn spec_embeds_the_explain_text() {
    let spec = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/POSE_SPEC.md")).unwrap();
    assert!(
        spec.replace("\r\n", "\n").contains(&handkit_core::explain::all()),
        "docs/POSE_SPEC.md is out of date: paste the output of `handkit explain` into its Reference section"
    );
}
