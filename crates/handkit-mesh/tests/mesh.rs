//! The generated hand at a coarse grid: watertight enough to render, skinned to the rig's
//! joints, with weights that sum to one and corrective shapes that stay local.

use handkit_core::pose::Side;
use handkit_mesh::{Options, generate, glb};

fn coarse(side: Side) -> handkit_mesh::HandMesh {
    generate(side, &Options { voxel: 0.004, textures: 256 })
}

#[test]
fn every_vertex_is_skinned_with_weights_summing_to_one() {
    let m = coarse(Side::Right);
    assert!(m.positions.len() > 1000);
    assert_eq!(m.joints.len(), m.positions.len());
    let n = m.skeleton.bones.len() as u16;
    for (j, w) in m.joints.iter().zip(&m.weights) {
        let sum: f32 = w.iter().sum();
        assert!((sum - 1.0).abs() < 1e-4, "weights sum to {sum}");
        assert!(j.iter().all(|x| *x < n));
    }
    assert!(m.indices.len() % 3 == 0 && m.indices.iter().all(|i| (*i as usize) < m.positions.len()));
}

#[test]
fn uv_seams_split_vertices_without_opening_the_surface() {
    let m = coarse(Side::Right);
    assert_eq!(m.uvs.len(), m.positions.len());
    assert!(m.uvs.iter().all(|t| (0.0..=1.0).contains(&t[0]) && (0.0..=1.0).contains(&t[1])));
    // Copies of a vertex share its position, skin weights and corrective offsets: weld them by
    // position and no edge is open.
    let mut id = std::collections::HashMap::new();
    let key = |p: [f32; 3]| p.map(f32::to_bits);
    let mut by_key: std::collections::HashMap<[u32; 3], usize> = std::collections::HashMap::new();
    for (i, p) in m.positions.iter().enumerate() {
        let first = *by_key.entry(key(*p)).or_insert(i);
        assert_eq!(m.joints[i], m.joints[first]);
        assert_eq!(m.weights[i], m.weights[first]);
        id.insert(i, first);
    }
    assert!(by_key.len() < m.positions.len(), "seams split some vertices");
    let mut edges: std::collections::HashMap<(usize, usize), u32> = std::collections::HashMap::new();
    for t in m.indices.chunks(3) {
        for k in 0..3 {
            let (a, b) = (id[&(t[k] as usize)], id[&(t[(k + 1) % 3] as usize)]);
            *edges.entry((a.min(b), a.max(b))).or_default() += 1;
        }
    }
    // (Surface nets can make an edge where two pairs of triangles meet: 4 is closed too.)
    let open = edges.values().filter(|c| *c % 2 == 1).count();
    assert_eq!(open, 0, "{open} edges are open");
    for t in &m.targets {
        let moved: std::collections::HashMap<usize, [f32; 3]> = t.iter().map(|(v, d)| (*v as usize, *d)).collect();
        for (v, d) in &moved {
            assert_eq!(moved.get(&id[v]), Some(d), "every copy moves the same");
        }
    }
    let maps = m.maps.as_ref().expect("baked maps");
    // (At 256 px the gutters take much of the atlas; at 2048 the charts cover about half.)
    assert!(maps.covered > 0.25, "the charts cover {:.0}% of the atlas", maps.covered * 100.0);
}

#[test]
fn left_hand_mirrors_the_right() {
    let (r, l) = (coarse(Side::Right), coarse(Side::Left));
    assert_eq!(r.positions.len(), l.positions.len());
    for (a, b) in r.positions.iter().zip(&l.positions).step_by(97) {
        assert!((a[0] + b[0]).abs() < 1e-6 && (a[1] - b[1]).abs() < 1e-6 && (a[2] - b[2]).abs() < 1e-6);
    }
    assert_eq!(r.targets.len(), l.targets.len());
}

#[test]
fn corrective_shapes_are_small_and_local() {
    let m = coarse(Side::Right);
    assert_eq!(m.targets.len(), handkit_core::rig::CORRECTIVES.len());
    for (t, (name, _)) in m.targets.iter().zip(handkit_core::rig::CORRECTIVES) {
        assert!(!t.is_empty(), "{name} moves no vertices");
        assert!(t.len() < m.positions.len() / 5, "{name} moves {} vertices", t.len());
        for (_, d) in t {
            let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
            assert!(len < 0.003, "{name} moves a vertex {len} m");
        }
    }
}

#[test]
fn glb_names_the_joints_and_targets() {
    let m = coarse(Side::Left);
    let bytes = glb::to_glb(&m);
    assert_eq!(&bytes[0..4], b"glTF");
    let len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let doc: serde_json::Value = serde_json::from_slice(&bytes[20..20 + len]).unwrap();
    let names: Vec<&str> = doc["nodes"].as_array().unwrap().iter().filter_map(|n| n["name"].as_str()).collect();
    assert!(names.contains(&"index_pip_L") && names.contains(&"thumb_cmc_L") && names.contains(&"skin_L"));
    let mesh = &doc["meshes"][0];
    assert_eq!(mesh["primitives"][0]["targets"].as_array().unwrap().len(), handkit_core::rig::CORRECTIVES.len());
    assert_eq!(mesh["extras"]["targetNames"][0], "index_mcp_flex");
}

/// Triangles that turn over when the hand is posed (linear blend skinning plus the corrective
/// shapes, as renderers draw it), counting only those off the palm side, which a fist hides.
fn turned_over_outside(m: &handkit_mesh::HandMesh, pose: &str) -> usize {
    use glam::DVec3;
    let text = std::fs::read_to_string(format!("{}/../handkit-core/gestures/{pose}.json", env!("CARGO_MANIFEST_DIR"))).unwrap();
    let solved = handkit_core::solve(&text).unwrap();
    let h = solved.hands.iter().find(|h| h.side == Side::Right).unwrap();
    let skin: Vec<_> = h.bones.iter().zip(&m.skeleton.bones).map(|(p, b)| p.frame * b.frame.inverse()).collect();
    let v = |p: [f32; 3]| DVec3::new(p[0] as f64, p[1] as f64, p[2] as f64);
    let rest: Vec<DVec3> = m.positions.iter().map(|p| v(*p)).collect();
    let mut bind = rest.clone();
    for (t, w) in m.targets.iter().zip(h.corrective_weights()) {
        for (i, d) in t {
            bind[*i as usize] += v(*d) * w;
        }
    }
    let posed: Vec<DVec3> = (0..bind.len())
        .map(|i| (0..4).map(|k| skin[m.joints[i][k] as usize].transform_point3(bind[i]) * m.weights[i][k] as f64).sum())
        .collect();
    let palmar = m.skeleton.bones.iter().find(|b| b.name == "hand").unwrap().frame.transform_vector3(DVec3::Z).normalize();
    let normal = |ps: &[DVec3], t: &[u32]| (ps[t[1] as usize] - ps[t[0] as usize]).cross(ps[t[2] as usize] - ps[t[0] as usize]);
    m.indices
        .chunks(3)
        .filter(|t| {
            let a = normal(&rest, t);
            let a_posed = skin[m.joints[t[0] as usize][0] as usize].transform_vector3(a);
            a_posed.dot(normal(&posed, t)) < 0.0 && a.normalize().dot(palmar) < 0.3
        })
        .count()
}

#[test]
fn a_fist_and_a_point_keep_the_knuckles_and_webs_closed() {
    // At full resolution, where the cracks showed: fingers that come together and bend must not
    // turn the skin of the webs and knuckles between them over.
    let m = generate(Side::Right, &Options { voxel: 0.0012, textures: 0 });
    for (pose, before) in [("fist", 119), ("point", 103)] {
        let n = turned_over_outside(&m, pose);
        assert!(n < 30, "{pose}: {n} triangles turn over off the palm side (were {before} before the weights were smoothed)");
    }
}

#[test]
fn skin_tones_keep_their_detail() {
    use handkit_mesh::skin::{BAKED_TONE, linear, nail_color, tone_albedo, tone_base};
    let baked = linear(BAKED_TONE);
    // The baked tone rebuilds to itself.
    let m = [0.4, 0.0, 0.3, 0.5];
    let out = tone_albedo([0.3, 0.2, 0.15], m, baked, nail_color(baked));
    assert!(out.iter().zip([0.3, 0.2, 0.15]).all(|(a, b)| (a - b).abs() < 1e-5), "{out:?}");
    // Darker skin: the palm and the nail bed stay much lighter than the back of the hand.
    let lum = |c: [f32; 3]| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
    for (name, rgb, ratio) in [("medium", [0xd8, 0xa6, 0x8a], 1.0), ("dark", [0x5e, 0x3e, 0x30], 2.0)] {
        let t = linear(rgb);
        let base = |m: [f32; 4]| tone_base(t, nail_color(t), m);
        let (back, palm, nail) = (base([0.0; 4]), base([0.0, 0.0, 0.0, 1.0]), base([0.0, 1.0, 0.0, 0.0]));
        assert!(lum(palm) >= lum(back) * ratio * 0.99, "{name}: palm {palm:?} back {back:?}");
        assert!(lum(nail) > lum(back) * ratio, "{name}: nail {nail:?}");
        // Redness and creases still show.
        let (red, crease) = (base([1.0, 0.0, 0.0, 0.0]), base([0.0, 0.0, 1.0, 0.0]));
        assert!(red[0] / red[1] > back[0] / back[1] * 1.05 && lum(crease) < lum(back) * 0.8, "{name}");
    }
}
