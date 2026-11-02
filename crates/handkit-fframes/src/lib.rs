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
//!
//! More people: one instance per hand, by its name: `right.add_as(&mut scene, &HandId::new("ana",
//! Side::Right))` poses the hand "ana.right" of each solved pose.

pub mod look;

pub use fframes_three_d as three_d;
pub use handkit_core::pose::{HandId, Side};

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
    /// TEXCOORD_1.x; all 0 when the file has none).
    pub nail: Arc<[f32]>,
}

/// A hand in one scene.
pub struct Hand {
    pub instance: ModelInstance,
    pub side: Side,
    /// The pose's hand it shows ("right", "ana.left").
    pub id: HandId,
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
        let nail = glb_texcoord1_x(bytes).filter(|n| n.len() == vertices).unwrap_or_else(|| vec![0.0; vertices]);
        Ok(HandModel { model, side, joint_names, nail: nail.into() })
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

    /// Adds the default person's hand of this side.
    pub fn add_to(&self, scene: &mut Scene) -> Hand {
        self.add_as(scene, &HandId::of(self.side))
    }

    /// Adds an instance that shows hand `id` of the poses (of this model's side).
    pub fn add_as(&self, scene: &mut Scene, id: &HandId) -> Hand {
        debug_assert_eq!(id.side, self.side, "a {} hand model shows {} hands", self.side.name(), self.side.name());
        let instance = self.model.add_to(scene, None);
        let joints = self.joint_names.iter().map(|n| instance.find(scene, n)).collect();
        let mesh = self.mesh().map(|id| instance.node(id));
        Hand { instance, side: self.side, id: id.clone(), joints, mesh }
    }
}

fn skinned_mesh(scene: &Scene) -> Option<NodeId> {
    scene.nodes().find(|(_, n)| n.as_mesh().is_some_and(|m| m.skin.is_some())).map(|(id, _)| id)
}

/// The x of TEXCOORD_1 of the first mesh primitive of a binary glTF (Handkit's nail mask).
fn glb_texcoord1_x(glb: &[u8]) -> Option<Vec<f32>> {
    let u32_at = |i: usize| Some(u32::from_le_bytes(glb.get(i..i + 4)?.try_into().ok()?) as usize);
    if glb.get(0..4)? != b"glTF" {
        return None;
    }
    let json_len = u32_at(12)?;
    let json: serde_json::Value = serde_json::from_slice(glb.get(20..20 + json_len)?).ok()?;
    let bin_start = 20 + json_len + 8;
    let bin = glb.get(bin_start..bin_start + u32_at(20 + json_len)?)?;
    let accessor = &json["accessors"][json["meshes"][0]["primitives"][0]["attributes"]["TEXCOORD_1"].as_u64()? as usize];
    if accessor["componentType"].as_u64()? != 5126 || accessor["type"].as_str()? != "VEC2" {
        return None;
    }
    let view = &json["bufferViews"][accessor["bufferView"].as_u64()? as usize];
    let offset = view["byteOffset"].as_u64().unwrap_or(0) as usize + accessor["byteOffset"].as_u64().unwrap_or(0) as usize;
    let stride = view["byteStride"].as_u64().unwrap_or(8) as usize;
    (0..accessor["count"].as_u64()? as usize)
        .map(|i| Some(f32::from_le_bytes(bin.get(offset + i * stride..offset + i * stride + 4)?.try_into().ok()?)))
        .collect()
}

impl Hand {
    /// The skinned mesh node.
    pub fn mesh(&self) -> Option<NodeId> {
        self.mesh
    }

    /// Poses the hand from the matching hand of `solved` (a pose without this hand leaves it
    /// as it is).
    pub fn pose(&self, scene: &mut Scene, solved: &Solved) {
        let Some(h) = solved.hands.iter().find(|h| h.id == self.id) else { return };
        for ((local, id), bone) in h.locals().iter().zip(&self.joints).zip(&h.bones) {
            let Some(id) = id else { continue };
            let t = local.translation;
            let r = local.rotation;
            scene[*id].position = Vec3::new(t.x as f32, t.y as f32, t.z as f32);
            scene[*id].rotation = Quat::from_xyzw(r.x as f32, r.y as f32, r.z as f32, r.w as f32);
            // A smaller or larger person's hand: the root scales the whole skeleton.
            if bone.parent.is_none() {
                scene[*id].scale = Vec3::splat(h.scale as f32);
            }
        }
        if let Some(mesh) = self.mesh.and_then(|id| scene[id].mesh_mut()) {
            mesh.morph_weights = h.corrective_weights().iter().map(|w| *w as f32).collect();
        }
    }
}
