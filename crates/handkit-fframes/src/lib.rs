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

pub use fframes_three_d as three_d;
pub use handkit_core::pose::Side;

use fframes_three_d::{Model, ModelError, ModelInstance, NodeId, Quat, Scene, Vec3};
use handkit_core::Solved;

/// A hand mesh, loaded once.
#[derive(Clone)]
pub struct HandModel {
    pub model: Model,
    pub side: Side,
    joint_names: Vec<String>,
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
        Ok(HandModel { model, side, joint_names })
    }

    /// Generates the procedural hand (about a second in a release build).
    pub fn generate(side: Side) -> Self {
        let mesh = handkit_mesh::generate(side, &handkit_mesh::Options::default());
        Self::from_glb(&handkit_mesh::glb::to_glb(&mesh)).expect("the generated hand loads")
    }

    pub fn add_to(&self, scene: &mut Scene) -> Hand {
        let instance = self.model.add_to(scene, None);
        let joints = self.joint_names.iter().map(|n| instance.find(scene, n)).collect();
        let mesh = (instance.offset..instance.offset + instance.len)
            .map(NodeId)
            .find(|id| scene.get(*id).and_then(|n| n.as_mesh()).is_some_and(|m| m.skin.is_some()));
        Hand { instance, side: self.side, joints, mesh }
    }
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
