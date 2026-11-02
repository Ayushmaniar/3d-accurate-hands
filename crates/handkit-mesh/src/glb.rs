//! Writes a [`HandMesh`] as a binary glTF 2.0 file: one skinned mesh and the rig's skeleton,
//! with joints named after the rig's bones (`index_pip_R`).

use serde_json::{Value, json};

use crate::HandMesh;

const FLOAT: u32 = 5126;
const USHORT: u32 = 5123;
const UINT: u32 = 5125;
const ARRAY_BUFFER: u32 = 34962;
const ELEMENT_ARRAY_BUFFER: u32 = 34963;

struct Builder {
    bin: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
}

impl Builder {
    fn push(&mut self, bytes: &[u8], target: Option<u32>) -> usize {
        while self.bin.len() % 4 != 0 {
            self.bin.push(0);
        }
        let mut view = json!({ "buffer": 0, "byteOffset": self.bin.len(), "byteLength": bytes.len() });
        if let Some(t) = target {
            view["target"] = json!(t);
        }
        self.bin.extend_from_slice(bytes);
        self.views.push(view);
        self.views.len() - 1
    }

    fn accessor(&mut self, bytes: &[u8], target: Option<u32>, component: u32, count: usize, kind: &str, extra: Value) -> usize {
        let view = self.push(bytes, target);
        let mut a = json!({ "bufferView": view, "componentType": component, "count": count, "type": kind });
        if let Value::Object(m) = extra {
            a.as_object_mut().unwrap().extend(m);
        }
        self.accessors.push(a);
        self.accessors.len() - 1
    }
}

fn bytes<T: Copy, const N: usize>(items: &[[T; N]]) -> Vec<u8> {
    let size = std::mem::size_of::<T>() * N;
    // SAFETY: plain arrays of f32/u16/u32, read as bytes in native (little-endian) order.
    unsafe { std::slice::from_raw_parts(items.as_ptr() as *const u8, items.len() * size) }.to_vec()
}

pub fn to_glb(mesh: &HandMesh) -> Vec<u8> {
    let mut b = Builder { bin: Vec::new(), views: Vec::new(), accessors: Vec::new() };
    let n = mesh.positions.len();
    let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
    for p in &mesh.positions {
        for i in 0..3 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    let position = b.accessor(&bytes(&mesh.positions), Some(ARRAY_BUFFER), FLOAT, n, "VEC3", json!({ "min": lo, "max": hi }));
    let normal = b.accessor(&bytes(&mesh.normals), Some(ARRAY_BUFFER), FLOAT, n, "VEC3", json!({}));
    let color = b.accessor(&bytes(&mesh.colors), Some(ARRAY_BUFFER), FLOAT, n, "VEC3", json!({}));
    // The nail mask rides in a standard slot, TEXCOORD_1.x, so every glTF loader accepts it.
    let nail: Vec<[f32; 2]> = mesh.nail.iter().map(|x| [*x, 0.0]).collect();
    let nail = b.accessor(&bytes(&nail), Some(ARRAY_BUFFER), FLOAT, n, "VEC2", json!({}));
    let joints = b.accessor(&bytes(&mesh.joints), Some(ARRAY_BUFFER), USHORT, n, "VEC4", json!({}));
    let weights = b.accessor(&bytes(&mesh.weights), Some(ARRAY_BUFFER), FLOAT, n, "VEC4", json!({}));
    let tris: Vec<[u32; 1]> = mesh.indices.iter().map(|x| [*x]).collect();
    let indices = b.accessor(&bytes(&tris), Some(ELEMENT_ARRAY_BUFFER), UINT, tris.len(), "SCALAR", json!({}));

    // Corrective shapes as sparse morph targets: only the vertices each one moves are stored.
    let mut target_accessors = Vec::new();
    for t in &mesh.targets {
        let idx: Vec<[u32; 1]> = t.iter().map(|(i, _)| [*i]).collect();
        let val: Vec<[f32; 3]> = t.iter().map(|(_, d)| *d).collect();
        let (mut lo, mut hi) = ([0f32; 3], [0f32; 3]);
        for d in &val {
            for i in 0..3 {
                lo[i] = lo[i].min(d[i]);
                hi[i] = hi[i].max(d[i]);
            }
        }
        let iv = b.push(&bytes(&idx), None);
        let vv = b.push(&bytes(&val), None);
        b.accessors.push(json!({
            "componentType": FLOAT, "count": n, "type": "VEC3", "min": lo, "max": hi,
            "sparse": { "count": t.len(), "indices": { "bufferView": iv, "componentType": UINT }, "values": { "bufferView": vv } }
        }));
        target_accessors.push(json!({ "POSITION": b.accessors.len() - 1 }));
    }
    let target_names: Vec<String> = handkit_core::rig::CORRECTIVES.iter().map(|(name, _)| format!("{name}_flex")).collect();

    let h = &mesh.skeleton;
    let ibm: Vec<[f32; 16]> = h.bones.iter().map(|bone| bone.frame.inverse().to_cols_array_2d()).map(|c| {
        // DAffine3 columns -> column-major 4x4.
        [
            c[0][0] as f32, c[0][1] as f32, c[0][2] as f32, 0.0,
            c[1][0] as f32, c[1][1] as f32, c[1][2] as f32, 0.0,
            c[2][0] as f32, c[2][1] as f32, c[2][2] as f32, 0.0,
            c[3][0] as f32, c[3][1] as f32, c[3][2] as f32, 1.0,
        ]
    }).collect();
    let ibm = b.accessor(&bytes(&ibm), None, FLOAT, h.bones.len(), "MAT4", json!({}));

    let locals = h.locals();
    let mut nodes: Vec<Value> = h
        .bones
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let l = locals[i];
            let children: Vec<usize> = h.bones.iter().enumerate().filter(|(_, c)| c.parent == Some(i)).map(|(j, _)| j).collect();
            let mut node = json!({
                "name": h.joint_name(i),
                "translation": [l.translation.x as f32, l.translation.y as f32, l.translation.z as f32],
                "rotation": [l.rotation.x as f32, l.rotation.y as f32, l.rotation.z as f32, l.rotation.w as f32],
            });
            if !children.is_empty() {
                node["children"] = json!(children);
            }
            node
        })
        .collect();
    let suffix = if mesh.side == handkit_core::pose::Side::Left { "L" } else { "R" };
    let mesh_node = nodes.len();
    // Not "hand_R": that is the hand joint, and loaders rename duplicate names.
    nodes.push(json!({ "name": format!("skin_{suffix}"), "mesh": 0, "skin": 0 }));
    let root = h.bones.iter().position(|b| b.parent.is_none()).unwrap();

    while b.bin.len() % 4 != 0 {
        b.bin.push(0);
    }
    let doc = json!({
        "asset": { "version": "2.0", "generator": "handkit-mesh" },
        "scene": 0,
        "scenes": [{ "nodes": [root, mesh_node] }],
        "nodes": nodes,
        "skins": [{ "name": format!("skeleton_{suffix}"), "joints": (0..h.bones.len()).collect::<Vec<_>>(), "inverseBindMatrices": ibm, "skeleton": root }],
        "meshes": [{
            "name": format!("skin_{suffix}"),
            "primitives": [{
                "attributes": { "POSITION": position, "NORMAL": normal, "COLOR_0": color, "TEXCOORD_1": nail, "JOINTS_0": joints, "WEIGHTS_0": weights },
                "indices": indices,
                "material": 0,
                "targets": target_accessors
            }],
            "weights": vec![0.0; target_names.len()],
            "extras": { "targetNames": target_names }
        }],
        "materials": [{ "name": "skin", "pbrMetallicRoughness": { "baseColorFactor": [1.0, 1.0, 1.0, 1.0], "metallicFactor": 0.0, "roughnessFactor": 0.55 } }],
        "buffers": [{ "byteLength": b.bin.len() }],
        "bufferViews": b.views,
        "accessors": b.accessors,
    });
    let mut text = serde_json::to_vec(&doc).unwrap();
    while text.len() % 4 != 0 {
        text.push(b' ');
    }
    let total = 12 + 8 + text.len() + 8 + b.bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(text.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&text);
    out.extend_from_slice(&(b.bin.len() as u32).to_le_bytes());
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(&b.bin);
    out
}
