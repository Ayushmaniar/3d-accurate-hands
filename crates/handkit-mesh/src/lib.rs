//! The procedural Handkit hand: a skinned mesh generated from the rig's own proportions, so the
//! skin and the capsules agree, written as a binary glTF that fframes and three.js both load.
//!
//! The mesh is bound in a slightly spread pose (fingers apart so they do not fuse) and its
//! skeleton is the rig's: one joint per `Bone`, named `<bone>_R` / `<bone>_L`. To pose it, set
//! every joint's local rotation and translation from [`handkit_core::Solved::skeleton`].

pub mod glb;
pub mod sdf;

use glam::{DVec3, dvec3};
use handkit_core::pose::Side;
use handkit_core::rig::HandGeom;
use serde_json::json;

/// The bind pose: the rig at rest with the fingers spread a little.
pub fn bind_pose(side: Side) -> serde_json::Value {
    let hand = json!({
        "place": { "elbow": [0, 0, 0] },
        "index": { "mcp_abd": 8 }, "ring": { "mcp_abd": 6 }, "pinky": { "mcp_abd": 12 }
    });
    json!({ "version": 1, "hands": { side.name(): hand } })
}

pub fn bind_geom(side: Side) -> HandGeom {
    let solved = handkit_core::solve_value(&bind_pose(side)).expect("bind pose");
    solved.hands.into_iter().next().unwrap()
}

pub struct HandMesh {
    pub side: Side,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    /// Skin tone, with nails lighter (linear RGB; SKIN and NAIL are sRGB).
    pub colors: Vec<[f32; 3]>,
    /// 1 on the nail plates, 0 elsewhere, for looks that shade nails apart.
    pub nail: Vec<f32>,
    pub joints: Vec<[u16; 4]>,
    pub weights: Vec<[f32; 4]>,
    pub indices: Vec<u32>,
    /// Corrective shapes in `rig::CORRECTIVES` order: (vertex, offset in bind space) for the
    /// vertices each one moves.
    pub targets: Vec<Vec<(u32, [f32; 3])>>,
    /// The bind skeleton (the rig's bones at the bind pose).
    pub skeleton: HandGeom,
}

pub struct Options {
    /// Grid spacing over the hand in meters (the forearm's grid is coarser along its length);
    /// 0.0012 gives about 130k triangles.
    pub voxel: f64,
}

impl Default for Options {
    fn default() -> Self {
        Options { voxel: 0.0012 }
    }
}

/// `f` over `items` on every core, in order.
fn par_map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let threads = std::thread::available_parallelism().map(|x| x.get()).unwrap_or(4);
    let chunk = items.len().div_ceil(threads).max(1);
    std::thread::scope(|s| {
        let f = &f;
        let parts: Vec<_> = items.chunks(chunk).map(|c| s.spawn(move || c.iter().map(f).collect::<Vec<R>>())).collect();
        parts.into_iter().flat_map(|p| p.join().unwrap()).collect()
    })
}

pub const SKIN: [f32; 3] = [0.86, 0.64, 0.53];
pub const NAIL: [f32; 3] = [0.95, 0.80, 0.76];

pub fn generate(side: Side, opts: &Options) -> HandMesh {
    // The shape is built for the right hand and reflected for the left; the left skeleton is
    // the rig's own left bind skeleton, which is the same reflection.
    let right = bind_geom(Side::Right);
    let field = sdf::hand_field(&right);
    let (positions, mut indices) = surface_nets(&field, &right, opts.voxel);
    // Pull each vertex onto the surface.
    let mut positions = par_map(&positions, |p| {
        let mut p = *p;
        for _ in 0..3 {
            let d = field.eval(p);
            p -= field.gradient(p) * d;
        }
        p
    });
    let normals: Vec<DVec3> = par_map(&positions, |p| field.gradient(*p));
    let band = 0.0025;
    let skin: Vec<([u16; 4], [f32; 4])> = par_map(&positions, |p| {
        let w = sdf::weights(&field, &right, *p, band);
        let mut j4 = [0u16; 4];
        let mut w4 = [0f32; 4];
        for (i, (b, x)) in w.iter().enumerate() {
            j4[i] = *b as u16;
            w4[i] = *x as f32;
        }
        (j4, w4)
    });
    let (joints, weights): (Vec<_>, Vec<_>) = skin.into_iter().unzip();
    let nail: Vec<f32> = par_map(&positions, |p| field.nail_mask(*p) as f32);
    // glTF vertex colors are linear.
    let linear = |c: f32| if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) };
    let colors = nail.iter().map(|&m| std::array::from_fn(|i| linear(SKIN[i] + (NAIL[i] - SKIN[i]) * m))).collect();

    let mut targets = correctives(&right, &positions);

    let skeleton = if side == Side::Left {
        for t in targets.iter_mut().flatten() {
            t.1[0] = -t.1[0];
        }
        for p in positions.iter_mut() {
            p.x = -p.x;
        }
        for t in indices.chunks_mut(3) {
            t.swap(1, 2);
        }
        bind_geom(Side::Left)
    } else {
        right
    };
    let flip = if side == Side::Left { -1.0 } else { 1.0 };
    HandMesh {
        side,
        positions: positions.iter().map(|p| [p.x as f32, p.y as f32, p.z as f32]).collect(),
        normals: normals.iter().map(|n| [(n.x * flip) as f32, n.y as f32, n.z as f32]).collect(),
        colors,
        nail,
        joints,
        weights,
        indices,
        targets,
        skeleton,
    }
}

/// Each corrective shape: around the joint, the back of the finger bulges out and the palm
/// side tucks in, fading out over about one finger radius along the bone.
fn correctives(h: &HandGeom, positions: &[DVec3]) -> Vec<Vec<(u32, [f32; 3])>> {
    handkit_core::rig::CORRECTIVES
        .iter()
        .map(|(name, _)| {
            let b = h.bones.iter().find(|b| b.name == *name).unwrap();
            let (digit, joint) = name.split_once('_').unwrap();
            let seg = match joint {
                "mcp" => "proximal",
                "pip" => "middle",
                _ => "distal",
            };
            let r = h.capsules.iter().find(|c| c.label() == format!("right.{digit}.{seg}")).map(|c| c.r).unwrap_or(0.009);
            let knuckle = joint == "mcp" && digit != "thumb";
            let (bulge, tuck, reach) = if knuckle { (0.22 * r, 0.12 * r, 1.5 * r) } else { (0.14 * r, 0.12 * r, 1.2 * r) };
            let to_local = b.frame.inverse();
            let mut out = Vec::new();
            for (i, p) in positions.iter().enumerate() {
                let q = to_local.transform_point3(*p);
                let rho = (q.x * q.x + q.z * q.z).sqrt();
                if q.y.abs() >= reach || rho < 1e-9 || rho > 1.8 * r {
                    continue;
                }
                let along = (1.0 - (q.y / reach).powi(2)).powi(2);
                // +1 on the palm side, -1 on the back; local -z points out of the back.
                let facing = q.z / rho;
                let push = if facing < 0.0 { -bulge * facing * facing } else { -tuck * facing * facing };
                let d = b.frame.transform_vector3(DVec3::new(0.0, 0.0, push * along));
                if d.length() > 1e-5 {
                    out.push((i as u32, [d.x as f32, d.y as f32, d.z as f32]));
                }
            }
            out
        })
        .collect()
}

/// Surface nets: one vertex per grid cell the surface crosses, one quad per crossing edge.
fn surface_nets(field: &sdf::Field, h: &HandGeom, voxel: f64) -> (Vec<DVec3>, Vec<u32>) {
    let (mut lo, mut hi) = (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY));
    for c in &h.capsules {
        for p in [c.a, c.b] {
            lo = lo.min(p - DVec3::splat(c.r));
            hi = hi.max(p + DVec3::splat(c.r));
        }
    }
    lo -= DVec3::splat(0.02);
    hi += DVec3::splat(0.02);
    // Along the forearm the surface barely curves: there the grid's rows are up to 2.5 times
    // farther apart, easing to the full resolution 3 cm before the wrist.
    let fine_from = h.bones.iter().find(|b| b.name == "hand").map(|b| b.origin.y).unwrap_or(lo.y) - 0.03;
    let mut ys = vec![lo.y];
    while *ys.last().unwrap() < hi.y {
        let y = *ys.last().unwrap();
        let t = ((fine_from - y) / 0.03).clamp(0.0, 1.0);
        ys.push(y + voxel * (1.0 + 1.5 * t * t * (3.0 - 2.0 * t)));
    }
    let n = ((hi - lo) / voxel).ceil().as_uvec3() + 1;
    let (nx, ny, nz) = (n.x as usize, ys.len(), n.z as usize);
    let at = |i: usize, j: usize, k: usize| dvec3(lo.x + i as f64 * voxel, ys[j], lo.z + k as f64 * voxel);
    let idx = |i: usize, j: usize, k: usize| (k * ny + j) * nx + i;

    // Sample the field, slab by slab on every core.
    let mut values = vec![0f64; nx * ny * nz];
    let threads = std::thread::available_parallelism().map(|x| x.get()).unwrap_or(4);
    let mut work: Vec<Vec<(usize, &mut [f64])>> = (0..threads).map(|_| Vec::new()).collect();
    for (k, slab) in values.chunks_mut(nx * ny).enumerate() {
        work[k % threads].push((k, slab));
    }
    std::thread::scope(|s| {
        for chunk in work {
            s.spawn(move || {
                for (k, slab) in chunk {
                    for j in 0..ny {
                        for i in 0..nx {
                            slab[j * nx + i] = field.eval(at(i, j, k));
                        }
                    }
                }
            });
        }
    });

    let cells = |i: usize, j: usize, k: usize| (k * (ny - 1) + j) * (nx - 1) + i;
    let mut cell_vertex = vec![u32::MAX; (nx - 1) * (ny - 1) * (nz - 1)];
    let mut positions = Vec::new();
    const CORNERS: [[usize; 3]; 8] = [[0, 0, 0], [1, 0, 0], [0, 1, 0], [1, 1, 0], [0, 0, 1], [1, 0, 1], [0, 1, 1], [1, 1, 1]];
    const EDGES: [[usize; 2]; 12] = [[0, 1], [2, 3], [4, 5], [6, 7], [0, 2], [1, 3], [4, 6], [5, 7], [0, 4], [1, 5], [2, 6], [3, 7]];
    for k in 0..nz - 1 {
        for j in 0..ny - 1 {
            for i in 0..nx - 1 {
                let v: [f64; 8] = std::array::from_fn(|c| values[idx(i + CORNERS[c][0], j + CORNERS[c][1], k + CORNERS[c][2])]);
                let inside = v.iter().filter(|x| **x < 0.0).count();
                if inside == 0 || inside == 8 {
                    continue;
                }
                let mut sum = DVec3::ZERO;
                let mut count = 0.0;
                for [a, b] in EDGES {
                    if (v[a] < 0.0) != (v[b] < 0.0) {
                        let t = v[a] / (v[a] - v[b]);
                        let pa = DVec3::new(CORNERS[a][0] as f64, CORNERS[a][1] as f64, CORNERS[a][2] as f64);
                        let pb = DVec3::new(CORNERS[b][0] as f64, CORNERS[b][1] as f64, CORNERS[b][2] as f64);
                        sum += pa + (pb - pa) * t;
                        count += 1.0;
                    }
                }
                cell_vertex[cells(i, j, k)] = positions.len() as u32;
                let size = dvec3(voxel, ys[j + 1] - ys[j], voxel);
                positions.push(at(i, j, k) + sum / count * size);
            }
        }
    }

    let mut indices = Vec::new();
    let mut quad = |q: [u32; 4], flip: bool, positions: &[DVec3]| {
        let q = if flip { [q[0], q[3], q[2], q[1]] } else { q };
        let p: [DVec3; 4] = q.map(|x| positions[x as usize]);
        // Split along the shorter diagonal.
        if p[0].distance_squared(p[2]) <= p[1].distance_squared(p[3]) {
            indices.extend([q[0], q[1], q[2], q[0], q[2], q[3]]);
        } else {
            indices.extend([q[0], q[1], q[3], q[1], q[2], q[3]]);
        }
    };
    for k in 0..nz {
        for j in 0..ny {
            for i in 0..nx {
                let inside = values[idx(i, j, k)] < 0.0;
                // Edges along x, y and z from this grid point; the four cells around each.
                if i + 1 < nx && j >= 1 && k >= 1 && j < ny - 1 && k < nz - 1 && (values[idx(i + 1, j, k)] < 0.0) != inside {
                    let c = [cells(i, j - 1, k - 1), cells(i, j, k - 1), cells(i, j, k), cells(i, j - 1, k)];
                    quad(c.map(|x| cell_vertex[x]), !inside, &positions);
                }
                if j + 1 < ny && i >= 1 && k >= 1 && i < nx - 1 && k < nz - 1 && (values[idx(i, j + 1, k)] < 0.0) != inside {
                    let c = [cells(i - 1, j, k - 1), cells(i - 1, j, k), cells(i, j, k), cells(i, j, k - 1)];
                    quad(c.map(|x| cell_vertex[x]), !inside, &positions);
                }
                if k + 1 < nz && i >= 1 && j >= 1 && i < nx - 1 && j < ny - 1 && (values[idx(i, j, k + 1)] < 0.0) != inside {
                    let c = [cells(i - 1, j - 1, k), cells(i, j - 1, k), cells(i, j, k), cells(i - 1, j, k)];
                    quad(c.map(|x| cell_vertex[x]), !inside, &positions);
                }
            }
        }
    }
    (positions, indices)
}
