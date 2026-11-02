//! The adapter finds the skinned mesh and its corrective shapes, and posing sets their weights.

use handkit_core::pose::Side;
use handkit_fframes::HandModel;
use handkit_fframes::three_d::Scene;

#[test]
fn pose_sets_the_corrective_weights() {
    let mesh = handkit_mesh::generate(Side::Right, &handkit_mesh::Options { voxel: 0.004, textures: 256 });
    let model = HandModel::from_glb(&handkit_mesh::glb::to_glb(&mesh)).unwrap();
    let mut scene = Scene::new();
    let hand = model.add_to(&mut scene);
    let fist = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../handkit-core/gestures/fist.json")).unwrap();
    hand.pose(&mut scene, &handkit_core::solve(&fist).unwrap());

    let id = hand.mesh().expect("the skinned mesh");
    let m = scene[id].as_mesh().unwrap();
    assert_eq!(m.geometry.morph_targets.len(), handkit_core::rig::CORRECTIVES.len());
    // The sparse targets decode to full arrays with the stored offsets in place.
    let moved = m.geometry.morph_targets[0].positions.iter().filter(|p| p.length() > 1e-5).count();
    assert_eq!(moved, mesh.targets[0].len());
    assert!(m.morph_weights[0] > 0.8, "a fist bends the index knuckle: {:?}", m.morph_weights);
}
