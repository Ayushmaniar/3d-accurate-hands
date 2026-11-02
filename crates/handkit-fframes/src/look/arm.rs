//! The forearm's end (a look's `arm`): the rig stops at the elbow, so the forearm either fades
//! out toward it or wears a rolled-up sleeve that covers it.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use fframes_three_d::{Geometry, Mat4, NodeId, Vec2, Vec3};

use super::img::{self, smoothstep};
use crate::HandModel;

/// A hand model's forearm in its bind pose.
pub(super) struct Forearm {
    /// The forearm joint, a node of the model's scene.
    pub joint: NodeId,
    /// Mesh space to the forearm's frame: the elbow at the origin, the forearm along +y.
    pub to_local: Mat4,
    /// From the elbow to the wrist, meters.
    pub len: f32,
}

/// The sleeve's cloth: its thickness over the skin, the bulge in the middle, the cuff's roll
/// (radius) and width, meters.
const THICK: f32 = 0.0035;
const BULGE: f32 = 0.0035;
const ROLL: f32 = 0.0055;
const CUFF: f32 = 0.022;
/// Points around the sleeve.
const AROUND: usize = 72;

impl Forearm {
    pub fn of(model: &HandModel) -> Option<Forearm> {
        let mesh = model.model.scene[model.mesh()?].as_mesh()?;
        let skin = mesh.skin.as_ref()?;
        let suffix = if model.side == crate::Side::Left { "L" } else { "R" };
        let ibm = |bone: &str| -> Option<(NodeId, Mat4)> {
            let name = format!("{bone}_{suffix}");
            let i = skin.joints.iter().position(|j| model.model.scene[*j].name.as_deref() == Some(name.as_str()))?;
            Some((skin.joints[i], *skin.inverse_bind_matrices.get(i)?))
        };
        let (joint, to_local) = ibm("forearm")?;
        let (_, hand) = ibm("hand")?;
        let len = to_local.transform_point3(hand.inverse().transform_point3(Vec3::ZERO)).length();
        Some(Forearm { joint, to_local, len })
    }

    /// The opacity of a fading arm at a point of the mesh: 0 at the elbow, 1 from 30% of the
    /// way to the wrist on.
    pub fn fade(&self, p: Vec3) -> f32 {
        smoothstep(0.01, 0.3 * self.len, self.to_local.transform_point3(p).y)
    }

    /// A rolled-up sleeve over this forearm (`skin`: the hand's bind positions), in the
    /// forearm's frame (a child of its joint): a slightly bulging tube with soft folds, closed
    /// past the elbow, reaching `length` toward the wrist and ending in a rolled cuff that
    /// tucks under into the arm; and each vertex's opacity (the end past the elbow fades out).
    pub fn sleeve(&self, skin: &[Vec3], length: f32) -> (Geometry, Vec<f32>) {
        let length = length.clamp(0.02, self.len - 0.04);
        // The forearm's section per slice along it: its extent in x and z (bind space).
        const STEP: f32 = 0.008;
        let n = (self.len / STEP) as usize + 1;
        let mut lo = vec![Vec2::splat(f32::INFINITY); n];
        let mut hi = vec![Vec2::splat(f32::NEG_INFINITY); n];
        let mut end = 0f32;
        for p in skin {
            let q = self.to_local.transform_point3(*p);
            end = end.min(q.y);
            if (0.0..self.len - 0.03).contains(&q.y) {
                let i = (q.y / STEP) as usize;
                lo[i] = lo[i].min(Vec2::new(q.x, q.z));
                hi[i] = hi[i].max(Vec2::new(q.x, q.z));
            }
        }
        let slices: Vec<(f32, Vec2, Vec2)> =
            (0..n).filter(|i| lo[*i].x.is_finite()).map(|i| ((i as f32 + 0.5) * STEP, (lo[i] + hi[i]) / 2.0, (hi[i] - lo[i]) / 2.0)).collect();
        // Centre and half extents at y, between the slices.
        let section = |y: f32| -> (Vec2, Vec2) {
            let Some(first) = slices.first() else { return (Vec2::ZERO, Vec2::splat(0.035)) };
            let k = slices.iter().position(|s| s.0 > y).unwrap_or(slices.len());
            match k {
                0 => (first.1, first.2),
                k if k == slices.len() => (slices[k - 1].1, slices[k - 1].2),
                k => {
                    let (a, b) = (slices[k - 1], slices[k]);
                    let t = (y - a.0) / (b.0 - a.0);
                    (a.1.lerp(b.1, t), a.2.lerp(b.2, t))
                }
            }
        };
        // Rings (y, how far out: 1 on the tube, less round the closed end, under the skin at
        // the tucked-in end): a short dome past the skin's own end, the tube, the cuff.
        let tube_end = end - 0.008;
        let mut rings: Vec<(f32, f32)> = (1..=8).map(|i| (i as f32 / 8.0) * FRAC_PI_2).map(|a| (tube_end - 0.012 * a.cos(), a.sin())).collect();
        let steps = ((length - tube_end) / 0.0015).ceil() as usize;
        rings.extend((1..=steps).map(|i| (tube_end + (length - tube_end) * i as f32 / steps as f32, 1.0)));
        rings.push((length + 0.001, -1.0));
        let cuff = CUFF.min(length * 0.45);
        let mut positions = vec![Vec3::new(section(0.0).0.x, tube_end - 0.012, section(0.0).0.y)];
        for (y, k) in &rings {
            let (c, a) = section(y.max(0.0));
            let tube = (y / (length - cuff)).clamp(0.0, 1.0);
            let u = ((y - (length - cuff)) / cuff).clamp(0.0, 1.0);
            let roll = ROLL * (1.0 - (2.0 * u - 1.0).powi(2)).max(0.0).sqrt();
            for j in 0..AROUND {
                let t = j as f32 / AROUND as f32 * TAU;
                let r = if *k < 0.0 {
                    a - Vec2::splat(0.003)
                } else {
                    // Soft, irregular folds on the tube, smoothing out into the cuff.
                    let folds = (img::fbm3(Vec3::new(t.cos() * 1.8, y * 50.0, t.sin() * 1.8)) - 0.5) * 0.008 * (1.0 - u);
                    (a + Vec2::splat(THICK + BULGE * (PI * tube).sin() + folds + roll)) * *k
                };
                positions.push(Vec3::new(c.x + r.x * t.cos(), *y, c.y + r.y * t.sin()));
            }
        }
        // Past the elbow the sleeve fades out, as if it went on up the arm.
        let opacity = positions.iter().map(|p| smoothstep(end, 0.012, p.y)).collect();
        let mut indices = Vec::new();
        let ring = |i: usize, j: usize| (1 + i * AROUND + j % AROUND) as u32;
        for j in 0..AROUND {
            indices.extend([0, ring(0, j), ring(0, j + 1)]);
        }
        for i in 0..rings.len() - 1 {
            for j in 0..AROUND {
                let (a, b, c, d) = (ring(i, j), ring(i, j + 1), ring(i + 1, j), ring(i + 1, j + 1));
                indices.extend([a, c, b, b, c, d]);
            }
        }
        (Geometry::new(positions, Vec::new(), Vec::new(), indices), opacity)
    }
}
