//! Handkit hands in fframes videos. Load a hand once, add it to each frame's scene, pose it
//! from a solved Handkit pose:
//!
//! ```rust,ignore
//! use handkit_fframes::{HandModel, three_d::*};
//! let right = HandModel::from_glb(include_bytes!("../assets/hand_R.glb"))?; // or HandModel::generate
//! // per frame
//! let pose = handkit_core::solve(&pose_json)?;          // or a clip sampled at t
//! let mut scene = self.stage.clone();
//! let hand = right.add_to(&mut scene);
//! hand.pose(&mut scene, &pose);
//! ```

pub mod look;

pub use fframes_three_d as three_d;
pub use handkit_core::pose::Side;

use std::sync::Arc;

use fframes_three_d::{Model, ModelError, ModelInstance, NodeId, Quat, Scene, Vec3};
use handkit_core::Solved;

/// A hand mesh, loaded once.
#[derive(Clone)]
pub struct HandModel {
    pub model: Model,
    pub side: Side,
    joint_names: Vec<String>,
    /// Per vertex of the skinned mesh: 1 on the nail plates, 0 elsewhere (the `.glb`'s
    /// TEXCOORD_1.x, or its mask map's G at each vertex; all 0 when the file has neither).
    pub nail: Arc<[f32]>,
    /// The baked skin maps, decoded for looks (a `.glb` from `handkit mesh` with textures).
    pub maps: Option<Arc<SkinImages>>,
}

/// The baked skin maps that looks rebuild their textures from (see `handkit_mesh::skin`).
pub struct SkinImages {
    /// The colour map, sRGB RGB bytes, `color_size` square. It shows `skin::BAKED_TONE`.
    pub color: Vec<[u8; 3]>,
    pub color_size: usize,
    /// R redness, G nail, B cavity (the mask map) and A palm skin (B of the occlusion /
    /// roughness map), linear bytes, `mask_size` square.
    pub masks: Vec<[u8; 4]>,
    pub mask_size: usize,
}

impl SkinImages {
    /// The masks at `uv`, bilinear, 0..1: (redness, nail, cavity, palm).
    pub fn masks_at(&self, uv: [f32; 2]) -> [f32; 4] {
        let n = self.mask_size;
        let (x, y) = (uv[0] * n as f32 - 0.5, uv[1] * n as f32 - 0.5);
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let at = |dx: f32, dy: f32| {
            let (i, j) = (((x0 + dx) as isize).clamp(0, n as isize - 1) as usize, ((y0 + dy) as isize).clamp(0, n as isize - 1) as usize);
            self.masks[j * n + i]
        };
        let (a, b, c, d) = (at(0.0, 0.0), at(1.0, 0.0), at(0.0, 1.0), at(1.0, 1.0));
        std::array::from_fn(|k| {
            let top = a[k] as f32 * (1.0 - fx) + b[k] as f32 * fx;
            let bottom = c[k] as f32 * (1.0 - fx) + d[k] as f32 * fx;
            (top * (1.0 - fy) + bottom * fy) / 255.0
        })
    }
}

/// A hand in one scene.
pub struct Hand {
    pub instance: ModelInstance,
    pub side: Side,
    joints: Vec<Option<NodeId>>,
    /// The skinned mesh, which carries the corrective shapes' weights.
    mesh: Option<NodeId>,
}

impl HandModel {
    /// Loads a hand `.glb` from `handkit mesh`. The side is read from its joint names.
    pub fn from_glb(bytes: &[u8]) -> Result<Self, ModelError> {
        let model = Model::from_gltf(bytes)?;
        let left = model.scene.find("skin_L").is_some();
        let side = if left { Side::Left } else { Side::Right };
        let skeleton = handkit_mesh::bind_geom(side);
        let joint_names = (0..skeleton.bones.len()).map(|i| skeleton.joint_name(i)).collect();
        let vertices = skinned_mesh(&model.scene).map_or(0, |id| model.scene[id].as_mesh().unwrap().geometry.positions.len());
        let maps = glb_skin_images(bytes).map(Arc::new);
        let nail = glb_texcoord1_x(bytes).filter(|n| n.len() == vertices).or_else(|| {
            // The mask map's nail channel at each vertex.
            let (maps, mesh) = (maps.as_ref()?, skinned_mesh(&model.scene)?);
            let g = &model.scene[mesh].as_mesh()?.geometry;
            (g.uvs.len() == vertices).then(|| g.uvs.iter().map(|uv| maps.masks_at([uv.x, uv.y])[1]).collect())
        });
        let nail = nail.unwrap_or_else(|| vec![0.0; vertices]);
        Ok(HandModel { model, side, joint_names, nail: nail.into(), maps })
    }

    /// The skinned mesh node in `model.scene`.
    pub fn mesh(&self) -> Option<NodeId> {
        skinned_mesh(&self.model.scene)
    }

    /// Generates the procedural hand (about a second in a release build).
    pub fn generate(side: Side) -> Self {
        let mesh = handkit_mesh::generate(side, &handkit_mesh::Options::default());
        Self::from_glb(&handkit_mesh::glb::to_glb(&mesh)).expect("the generated hand loads")
    }

    pub fn add_to(&self, scene: &mut Scene) -> Hand {
        let instance = self.model.add_to(scene, None);
        let joints = self.joint_names.iter().map(|n| instance.find(scene, n)).collect();
        let mesh = self.mesh().map(|id| instance.node(id));
        Hand { instance, side: self.side, joints, mesh }
    }
}

fn skinned_mesh(scene: &Scene) -> Option<NodeId> {
    scene.nodes().find(|(_, n)| n.as_mesh().is_some_and(|m| m.skin.is_some())).map(|(id, _)| id)
}

/// The JSON and binary chunks of a binary glTF.
fn glb_chunks(glb: &[u8]) -> Option<(serde_json::Value, &[u8])> {
    let u32_at = |i: usize| Some(u32::from_le_bytes(glb.get(i..i + 4)?.try_into().ok()?) as usize);
    if glb.get(0..4)? != b"glTF" {
        return None;
    }
    let json_len = u32_at(12)?;
    let json: serde_json::Value = serde_json::from_slice(glb.get(20..20 + json_len)?).ok()?;
    let bin_start = 20 + json_len + 8;
    let bin = glb.get(bin_start..bin_start + u32_at(20 + json_len)?)?;
    Some((json, bin))
}

/// The skin maps of a `.glb` from `handkit mesh`, found by their image names.
fn glb_skin_images(glb: &[u8]) -> Option<SkinImages> {
    let (json, bin) = glb_chunks(glb)?;
    let image = |name: &str| -> Option<image::RgbImage> {
        let img = json["images"].as_array()?.iter().find(|i| i["name"] == name)?;
        let view = &json["bufferViews"][img["bufferView"].as_u64()? as usize];
        let offset = view["byteOffset"].as_u64().unwrap_or(0) as usize;
        let bytes = bin.get(offset..offset + view["byteLength"].as_u64()? as usize)?;
        Some(image::load_from_memory(bytes).ok()?.to_rgb8())
    };
    let (color, masks, orm) = (image("skin_color")?, image("skin_masks")?, image("skin_orm")?);
    if masks.dimensions() != orm.dimensions() || color.width() != color.height() || masks.width() != masks.height() {
        return None;
    }
    Some(SkinImages {
        color_size: color.width() as usize,
        color: color.pixels().map(|p| p.0).collect(),
        mask_size: masks.width() as usize,
        masks: masks.pixels().zip(orm.pixels()).map(|(m, o)| [m[0], m[1], m[2], o[2]]).collect(),
    })
}

/// The x of TEXCOORD_1 of the first mesh primitive of a binary glTF (Handkit's nail mask).
fn glb_texcoord1_x(glb: &[u8]) -> Option<Vec<f32>> {
    let (json, bin) = glb_chunks(glb)?;
    let accessor = &json["accessors"][json["meshes"][0]["primitives"][0]["attributes"]["TEXCOORD_1"].as_u64()? as usize];
    if accessor["type"].as_str()? != "VEC2" {
        return None;
    }
    // Floats, or normalized unsigned shorts (what `handkit mesh` writes).
    let size = match accessor["componentType"].as_u64()? {
        5126 => 4,
        5123 => 2,
        _ => return None,
    };
    let view = &json["bufferViews"][accessor["bufferView"].as_u64()? as usize];
    let offset = view["byteOffset"].as_u64().unwrap_or(0) as usize + accessor["byteOffset"].as_u64().unwrap_or(0) as usize;
    let stride = view["byteStride"].as_u64().unwrap_or(2 * size as u64) as usize;
    (0..accessor["count"].as_u64()? as usize)
        .map(|i| {
            let at = offset + i * stride;
            Some(match size {
                4 => f32::from_le_bytes(bin.get(at..at + 4)?.try_into().ok()?),
                _ => u16::from_le_bytes(bin.get(at..at + 2)?.try_into().ok()?) as f32 / 65535.0,
            })
        })
        .collect()
}

impl Hand {
    /// The skinned mesh node.
    pub fn mesh(&self) -> Option<NodeId> {
        self.mesh
    }

    /// Poses the hand from the matching hand of `solved` (a pose without this side leaves it
    /// as it is).
    pub fn pose(&self, scene: &mut Scene, solved: &Solved) {
        let Some(h) = solved.hands.iter().find(|h| h.side == self.side) else { return };
        for (local, id) in h.locals().iter().zip(&self.joints) {
            let Some(id) = id else { continue };
            let t = local.translation;
            let r = local.rotation;
            scene[*id].position = Vec3::new(t.x as f32, t.y as f32, t.z as f32);
            scene[*id].rotation = Quat::from_xyzw(r.x as f32, r.y as f32, r.z as f32, r.w as f32);
        }
        if let Some(mesh) = self.mesh.and_then(|id| scene[id].mesh_mut()) {
            mesh.morph_weights = h.corrective_weights().iter().map(|w| *w as f32).collect();
        }
    }
}
