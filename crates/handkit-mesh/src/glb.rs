//! Writes a [`HandMesh`] as a binary glTF 2.0 file: one skinned mesh and the rig's skeleton,
//! with joints named after the rig's bones (`index_pip_R`).

use serde_json::{Value, json};

use crate::HandMesh;

const FLOAT: u32 = 5126;
const USHORT: u32 = 5123;
const UBYTE: u32 = 5121;
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
    let unorm16 = |x: f32| (x.clamp(0.0, 1.0) * 65535.0).round() as u16;
    let mut attributes = json!({ "POSITION": position, "NORMAL": normal });
    if !mesh.uvs.is_empty() {
        let uv: Vec<[u16; 2]> = mesh.uvs.iter().map(|t| [unorm16(t[0]), unorm16(t[1])]).collect();
        attributes["TEXCOORD_0"] = json!(b.accessor(&bytes(&uv), Some(ARRAY_BUFFER), USHORT, n, "VEC2", json!({ "normalized": true })));
    }
    // With baked maps the colour comes from the textures; vertex colors would tint them.
    if mesh.maps.is_none() {
        attributes["COLOR_0"] = json!(b.accessor(&bytes(&mesh.colors), Some(ARRAY_BUFFER), FLOAT, n, "VEC3", json!({})));
    }
    // Without maps, the nail mask rides in a standard slot, TEXCOORD_1.x, so every glTF loader
    // accepts it (with them, it is the mask map's G).
    if mesh.maps.is_none() {
        let nail: Vec<[u16; 2]> = mesh.nail.iter().map(|x| [unorm16(*x), 0]).collect();
        attributes["TEXCOORD_1"] = json!(b.accessor(&bytes(&nail), Some(ARRAY_BUFFER), USHORT, n, "VEC2", json!({ "normalized": true })));
    }
    let joints: Vec<[u8; 4]> = mesh.joints.iter().map(|j| j.map(|x| x as u8)).collect();
    attributes["JOINTS_0"] = json!(b.accessor(&bytes(&joints), Some(ARRAY_BUFFER), UBYTE, n, "VEC4", json!({})));
    // Weights as bytes that still add up to exactly 255.
    let weights: Vec<[u8; 4]> = mesh
        .weights
        .iter()
        .map(|w| {
            let mut q = w.map(|x| (x * 255.0).round().clamp(0.0, 255.0) as i32);
            let first = (0..4).max_by(|&a, &b| w[a].total_cmp(&w[b])).unwrap();
            q[first] += 255 - q.iter().sum::<i32>();
            q.map(|x| x.clamp(0, 255) as u8)
        })
        .collect();
    attributes["WEIGHTS_0"] = json!(b.accessor(&bytes(&weights), Some(ARRAY_BUFFER), UBYTE, n, "VEC4", json!({ "normalized": true })));
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

    // The skin maps: colour, normal, occlusion + roughness in one image (R and G), and the
    // tone-free masks, which no standard slot takes: the material's extras point at them.
    let mut material = json!({ "name": "skin", "pbrMetallicRoughness": { "baseColorFactor": [1.0, 1.0, 1.0, 1.0], "metallicFactor": 0.0, "roughnessFactor": 0.55 } });
    let mut images = Vec::new();
    if let Some(m) = &mesh.maps {
        for (data, mime, name) in [(&m.albedo, "image/jpeg", "skin_color"), (&m.normal, "image/jpeg", "skin_normal"), (&m.orm, "image/jpeg", "skin_orm"), (&m.masks, "image/jpeg", "skin_masks")] {
            let view = b.push(data, None);
            images.push(json!({ "name": name, "bufferView": view, "mimeType": mime }));
        }
        material = json!({
            "name": "skin",
            "pbrMetallicRoughness": {
                "baseColorFactor": [1.0, 1.0, 1.0, 1.0],
                "baseColorTexture": { "index": 0 },
                "metallicFactor": 0.0,
                "roughnessFactor": 1.0,
                "metallicRoughnessTexture": { "index": 2 }
            },
            "normalTexture": { "index": 1, "scale": 1.0 },
            "occlusionTexture": { "index": 2, "strength": 1.0 },
            "extras": { "handkit": { "masks": { "index": 3 }, "masksChannels": "R redness, G nail, B cavity; the palm skin mask is B of the metallic-roughness texture", "bakedTone": crate::skin::BAKED_TONE } }
        });
    }
    let textures: Vec<Value> = (0..images.len()).map(|i| json!({ "sampler": 0, "source": i })).collect();

    while b.bin.len() % 4 != 0 {
        b.bin.push(0);
    }
    let mut doc = json!({
        "asset": { "version": "2.0", "generator": "handkit-mesh" },
        "scene": 0,
        "scenes": [{ "nodes": [root, mesh_node] }],
        "nodes": nodes,
        "skins": [{ "name": format!("skeleton_{suffix}"), "joints": (0..h.bones.len()).collect::<Vec<_>>(), "inverseBindMatrices": ibm, "skeleton": root }],
        "meshes": [{
            "name": format!("skin_{suffix}"),
            "primitives": [{
                "attributes": attributes,
                "indices": indices,
                "material": 0,
                "targets": target_accessors
            }],
            "weights": vec![0.0; target_names.len()],
            "extras": { "targetNames": target_names }
        }],
        "materials": [material],
        "buffers": [{ "byteLength": b.bin.len() }],
        "bufferViews": b.views,
        "accessors": b.accessors,
    });
    if !images.is_empty() {
        // Linear filtering with mipmaps; clamped, since the atlas does not repeat.
        doc["samplers"] = json!([{ "magFilter": 9729, "minFilter": 9987, "wrapS": 33071, "wrapT": 33071 }]);
        doc["images"] = json!(images);
        doc["textures"] = json!(textures);
    }
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
