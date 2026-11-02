//! Textures in the hand's UV atlas that the looks build once per hand and per look and keep:
//! the colour map of a skin tone (from the baked maps), the data map of the stylized passes,
//! arcane's brush strokes and clay's lumps and fingerprints.
//!
//! Solid textures come from the bind-pose point under each texel (the mesh rasterized in UV
//! space), so they stay on the skin as it moves and do not break at the atlas' seams.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use fframes_three_d::{Geometry, Mat4, Texture, Vec2, Vec3};
use handkit_mesh::skin;
use rayon::prelude::*;

use super::img::{self, smoothstep};
use crate::{HandModel, SkinImages};

/// What a cached texture is.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Kind {
    /// The colour map of a skin tone and nail colour.
    Tone([u8; 3], [u8; 3]),
    /// The stylized data map: R nail and cavity, G palm skin.
    Data,
    /// Arcane's data map (G the strokes' warm / cool shift) and stroke normals.
    ArcaneData,
    ArcaneNormal,
    /// Clay's lumps and fingerprints, as normals.
    ClayNormal,
}

type Key = (usize, Kind);

/// Built textures by (hand, kind). The hand's geometry is kept alive with them, so its address
/// stays theirs.
#[allow(clippy::type_complexity)]
fn cache() -> &'static Mutex<HashMap<Key, (Arc<Geometry>, Arc<Texture>)>> {
    static CACHE: std::sync::OnceLock<Mutex<HashMap<Key, (Arc<Geometry>, Arc<Texture>)>>> = std::sync::OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// The texture `kind` for `model`, built on first use. `None` when the model has no UVs or
/// no baked maps for a kind that needs them.
pub(crate) fn texture(model: &HandModel, kind: Kind) -> Option<Arc<Texture>> {
    let mesh = model.mesh()?;
    let m = model.model.scene[mesh].as_mesh()?;
    let geometry = &m.geometry;
    if geometry.uvs.len() != geometry.positions.len() {
        return None;
    }
    let key = (Arc::as_ptr(geometry) as usize, kind);
    if let Some((_, t)) = cache().lock().unwrap().get(&key) {
        return Some(Arc::clone(t));
    }
    let maps = model.maps.as_deref();
    let ibm = m.skin.as_ref().map(|s| Arc::clone(&s.inverse_bind_matrices));
    let built = Arc::new(match kind {
        Kind::Tone(skin, nail) => tone(maps?, skin, nail),
        Kind::Data => {
            let maps = maps?;
            let atlas = Atlas::new(geometry, maps.mask_size);
            let px = data_pixels(Some(maps), &atlas);
            Texture::from_rgba8(atlas.size as u32, atlas.size as u32, &px, false)
        }
        Kind::ArcaneData | Kind::ArcaneNormal => {
            let atlas = Atlas::new(geometry, 1024);
            let ibm = ibm?;
            let strokes = |t: &Texel| strokes(t, &ibm);
            if kind == Kind::ArcaneData {
                let tint = atlas.field(geometry, |t| strokes(t).0);
                let mut px = data_pixels(maps, &atlas);
                for (p, v) in px.chunks_mut(4).zip(&tint) {
                    p[1] = (v.unwrap_or(0.5) * 255.0).round() as u8;
                }
                Texture::from_rgba8(atlas.size as u32, atlas.size as u32, &px, false)
            } else {
                atlas.normal_map(geometry, |t| strokes(t).1)
            }
        }
        Kind::ClayNormal => {
            let atlas = Atlas::new(geometry, 2048);
            atlas.normal_map(geometry, clay_height)
        }
    });
    cache().lock().unwrap().insert(key, (Arc::clone(geometry), Arc::clone(&built)));
    Some(built)
}

/// The colour map for a skin tone: the baked map rebuilt texel by texel from the masks.
fn tone(maps: &SkinImages, skin: [u8; 3], nail: [u8; 3]) -> Texture {
    let n = maps.color_size;
    let lut = img::srgb_to_linear_lut();
    let (tone, nail) = (skin::linear(skin), skin::linear(nail));
    let mut px = vec![0u8; n * n * 4];
    px.par_chunks_mut(n * 4).enumerate().for_each(|(y, row)| {
        for x in 0..n {
            let c = maps.color[y * n + x];
            let baked = c.map(|v| lut[v as usize]);
            let m = maps.masks_at([(x as f32 + 0.5) / n as f32, (y as f32 + 0.5) / n as f32]);
            let out = skin::tone_albedo(baked, m, tone, nail);
            for k in 0..3 {
                row[x * 4 + k] = (img::linear_to_srgb(out[k]) * 255.0).round() as u8;
            }
            row[x * 4 + 3] = 255;
        }
    });
    Texture::from_rgba8(n as u32, n as u32, &px, true)
}

/// R: the crease lines up to 0.45, and the nails from 0.55 up; ink lines and nail colours read
/// it back with [`unpack`]. (A nail's soft edge reads as a line, so nails get one.)
fn pack(nail: f32, line: f32) -> f32 {
    if nail > 0.5 { 0.55 + 0.45 * smoothstep(0.5, 1.0, nail) } else { 0.45 * line.max(nail * 2.0).min(1.0) }
}

/// A data R for a nail mask alone.
pub(crate) fn pack_nail(nail: f32) -> f32 {
    pack(nail, 0.0)
}

/// (nail, crease line) from a data map's R.
pub(crate) fn unpack(r: f32) -> (f32, f32) {
    let nail = smoothstep(0.5, 0.6, r);
    (nail, (r / 0.45).min(1.0) * (1.0 - nail))
}

/// The stylized data map's pixels: R nails and crease lines, G the palm skin. A crease line
/// is the ridge of the cavity map (smoothed inside the charts, so their borders do not draw):
/// where it curves down most steeply across, a soft line about a texel wide, centred on the
/// ridge to a fraction of a texel.
fn data_pixels(maps: Option<&SkinImages>, atlas: &Atlas) -> Vec<u8> {
    let size = atlas.size;
    let uv = |i: usize| [((i % size) as f32 + 0.5) / size as f32, ((i / size) as f32 + 0.5) / size as f32];
    let masks: Vec<[f32; 4]> = (0..size * size).into_par_iter().map(|i| maps.map_or([0.0; 4], |m| m.masks_at(uv(i)))).collect();
    let covered: Vec<f32> = atlas.tri.iter().map(|t| if *t == u32::MAX { 0.0 } else { 1.0 }).collect();
    let mut cavity: Vec<f32> = masks.iter().zip(&covered).map(|(m, c)| m[2] * c).collect();
    let mut weight = covered.clone();
    let sigma = 1.6 * size as f32 / 1024.0;
    img::blur(&mut cavity, size, size, sigma);
    img::blur(&mut weight, size, size, sigma);
    let smooth: Vec<f32> = cavity.iter().zip(&weight).map(|(c, w)| if *w > 1e-3 { c / w } else { 0.0 }).collect();
    let at = |x: usize, y: usize| smooth[y.min(size - 1) * size + x.min(size - 1)];
    let mut px = vec![0u8; size * size * 4];
    px.par_chunks_mut(4).enumerate().for_each(|(i, p)| {
        let (x, y) = (i % size, i / size);
        let m = masks[i];
        let mut line = 0.0;
        if covered[i] > 0.0 && x > 0 && y > 0 {
            let c = at(x, y);
            let (l, r, u, d) = (at(x - 1, y), at(x + 1, y), at(x, y - 1), at(x, y + 1));
            let (gx, gy) = ((r - l) * 0.5, (d - u) * 0.5);
            let (hxx, hyy) = (r - 2.0 * c + l, d - 2.0 * c + u);
            let hxy = (at(x + 1, y + 1) - at(x + 1, y - 1) - at(x - 1, y + 1) + at(x - 1, y - 1)) * 0.25;
            // The most negative curvature and its direction.
            let mean = 0.5 * (hxx + hyy);
            let root = (0.25 * (hxx - hyy).powi(2) + hxy * hxy).sqrt();
            let lambda = mean - root;
            if lambda < 0.0 {
                let (nx, ny) = if hxy.abs() > 1e-9 { (lambda - hyy, hxy) } else if hxx < hyy { (1.0, 0.0) } else { (0.0, 1.0) };
                let len = (nx * nx + ny * ny).sqrt().max(1e-9);
                let (nx, ny) = (nx / len, ny / len);
                // How far the ridge's crest is from this texel, across.
                let t = -(gx * nx + gy * ny) / lambda;
                line = (-(t / 0.75).powi(2)).exp() * smoothstep(0.004, 0.02, -lambda) * smoothstep(0.25, 0.5, c) * smoothstep(0.85, 0.98, weight[i]);
            }
        }
        p[0] = (pack(m[1], line) * 255.0).round() as u8;
        p[1] = (m[3] * 255.0).round() as u8;
        p[3] = 255;
    });
    px
}

/// The mesh rasterized into its UV atlas: each texel's triangle and barycentrics.
struct Atlas {
    size: usize,
    tri: Vec<u32>,
    bary: Vec<[f32; 2]>,
}

/// What a solid texture sees at a texel (bind pose).
struct Texel {
    p: Vec3,
    n: Vec3,
    joints: [u16; 4],
    weights: [f32; 4],
}

impl Atlas {
    fn new(g: &Geometry, size: usize) -> Atlas {
        let n = size as f32;
        let uv = |v: u32| g.uvs[v as usize] * n - Vec2::splat(0.5);
        let mut tri = vec![u32::MAX; size * size];
        let mut bary = vec![[0f32; 2]; size * size];
        for (t, c) in g.indices.chunks(3).enumerate() {
            let (a, b, cc) = (uv(c[0]), uv(c[1]), uv(c[2]));
            let det = (b - a).perp_dot(cc - a);
            if det.abs() < 1e-12 {
                continue;
            }
            let lo = a.min(b).min(cc).floor().max(Vec2::ZERO);
            let hi = a.max(b).max(cc).ceil().min(Vec2::splat(n - 1.0));
            for y in lo.y as usize..=hi.y as usize {
                for x in lo.x as usize..=hi.x as usize {
                    let p = Vec2::new(x as f32, y as f32);
                    let l1 = (p - a).perp_dot(cc - a) / det;
                    let l2 = (b - a).perp_dot(p - a) / det;
                    if l1 < -1e-4 || l2 < -1e-4 || l1 + l2 > 1.0 + 1e-4 {
                        continue;
                    }
                    tri[y * size + x] = t as u32;
                    bary[y * size + x] = [l1, l2];
                }
            }
        }
        // Grow the charts by two texels (their triangles extrapolated) for filtering.
        for _ in 0..2 {
            let mut add = Vec::new();
            for y in 0..size {
                for x in 0..size {
                    if tri[y * size + x] != u32::MAX {
                        continue;
                    }
                    let near = [(-1i64, 0i64), (1, 0), (0, -1), (0, 1)].iter().find_map(|(dx, dy)| {
                        let (xx, yy) = (x as i64 + dx, y as i64 + dy);
                        (xx >= 0 && yy >= 0 && (xx as usize) < size && (yy as usize) < size).then(|| tri[yy as usize * size + xx as usize]).filter(|t| *t != u32::MAX)
                    });
                    if let Some(t) = near {
                        let c = &g.indices[3 * t as usize..3 * t as usize + 3];
                        let (a, b, cc) = (uv(c[0]), uv(c[1]), uv(c[2]));
                        let det = (b - a).perp_dot(cc - a);
                        let p = Vec2::new(x as f32, y as f32);
                        add.push((y * size + x, t, [(p - a).perp_dot(cc - a) / det, (b - a).perp_dot(p - a) / det]));
                    }
                }
            }
            for (i, t, b) in add {
                tri[i] = t;
                bary[i] = b;
            }
        }
        Atlas { size, tri, bary }
    }

    fn texel(&self, g: &Geometry, i: usize) -> Option<(Texel, [u32; 3])> {
        let t = self.tri[i];
        if t == u32::MAX {
            return None;
        }
        let c = [0, 1, 2].map(|k| g.indices[3 * t as usize + k]);
        let [l1, l2] = self.bary[i];
        let l = [1.0 - l1 - l2, l1, l2];
        let p = c.iter().zip(l).map(|(v, w)| g.positions[*v as usize] * w).sum::<Vec3>();
        let n = c.iter().zip(l).map(|(v, w)| g.normals[*v as usize] * w).sum::<Vec3>().normalize_or(Vec3::Z);
        // The skin weights of the corner nearest the texel.
        let k = (0..3).max_by(|a, b| l[*a].total_cmp(&l[*b])).unwrap();
        let (joints, weights) = g.skin.as_ref().map_or(([0; 4], [1.0, 0.0, 0.0, 0.0]), |s| (s.joints[c[k] as usize], s.weights[c[k] as usize]));
        Some((Texel { p, n, joints, weights }, c))
    }

    /// `f` at every texel the charts cover.
    fn field(&self, g: &Geometry, f: impl Fn(&Texel) -> f32 + Sync) -> Vec<Option<f32>> {
        (0..self.size * self.size).into_par_iter().map(|i| self.texel(g, i).map(|(t, _)| f(&t))).collect()
    }

    /// A tangent-space normal map of the height `h` (meters) over the skin, in the frame the
    /// renderers build from the UVs (+U, and +V toward the bottom of the image).
    fn normal_map(&self, g: &Geometry, h: impl Fn(&Texel) -> f32 + Sync) -> Texture {
        let size = self.size;
        let mut px = vec![0u8; size * size * 4];
        px.par_chunks_mut(4).enumerate().for_each(|(i, out)| {
            out.copy_from_slice(&[128, 128, 255, 255]);
            let Some((t, c)) = self.texel(g, i) else { return };
            let (p0, p1, p2) = (g.positions[c[0] as usize], g.positions[c[1] as usize], g.positions[c[2] as usize]);
            let (u0, u1, u2) = (g.uvs[c[0] as usize], g.uvs[c[1] as usize], g.uvs[c[2] as usize]);
            let (e1, e2, d1, d2) = (p1 - p0, p2 - p0, u1 - u0, u2 - u0);
            let det = d1.perp_dot(d2);
            if det == 0.0 {
                return;
            }
            let sign = det.signum();
            let dpdu = (e1 * d2.y - e2 * d1.y) * sign;
            let dpdv = (e2 * d1.x - e1 * d2.x) * sign;
            let tu = (dpdu - t.n * t.n.dot(dpdu)).normalize_or_zero();
            let tv = (dpdv - t.n * t.n.dot(dpdv)).normalize_or_zero();
            // Half a texel along the surface.
            let e = (0.5 * dpdu.length() / det.abs() / size as f32).clamp(2e-5, 5e-4);
            let at = |d: Vec3| h(&Texel { p: t.p + d * e, ..t });
            let gu = (at(tu) - at(-tu)) / (2.0 * e);
            let gv = (at(tv) - at(-tv)) / (2.0 * e);
            // bent = n - tu gu - tv gv = tu mx - tv my + n mz.
            let m = Vec3::new(-gu, gv, 1.0).normalize();
            for k in 0..3 {
                out[k] = ((m[k] * 0.5 + 0.5) * 255.0).round() as u8;
            }
        });
        Texture::from_rgba8(size as u32, size as u32, &px, false)
    }
}

/// Arcane's paint at a texel: (warm / cool shift 0..1, 0.5 neutral; paint height in meters).
/// Short dabs laid along each bone (in the bone's rest frame, on the plane the skin faces), each
/// one flat in colour with bristle grooves along it; where bones or planes meet, whole strokes
/// of one or the other win, never a blend of both.
fn strokes(t: &Texel, ibm: &[Mat4]) -> (f32, f32) {
    // Pick a bone by weight, dithered by a coarse noise so the change follows strokes.
    let pick = img::noise3(t.p * 180.0 + Vec3::splat(3.7));
    let mut acc = 0.0;
    let mut bone = t.joints[0] as usize;
    for k in 0..4 {
        acc += t.weights[k];
        if pick * 0.999 < acc {
            bone = t.joints[k] as usize;
            break;
        }
    }
    let m = ibm.get(bone).copied().unwrap_or(Mat4::IDENTITY);
    let q = m.transform_point3(t.p);
    let n = m.transform_vector3(t.n).normalize_or(Vec3::Z);
    // The plane: across the bone on its back and front (z) or its sides (x); the bone's
    // length is always v. Ends of the fingers and the palm's edge use the x-z plane.
    let wobble = (img::noise3(t.p * 220.0) - 0.5) * 0.5;
    let (u, v, plane) = if n.y.abs() > 0.8 + wobble * 0.3 {
        (q.x, q.z, 2)
    } else if n.x.abs() > n.z.abs() + wobble {
        (q.z, q.y, 0)
    } else {
        (q.x, q.y, 1)
    };
    let p = Vec2::new(u, v);
    let (cw, ch) = (0.0034f32, 0.0075f32);
    let cell = Vec2::new((p.x / cw).floor(), (p.y / ch).floor());
    let seed = (bone * 3 + plane) as i32;
    let (mut best, mut tint, mut height) = (-1.0f32, 0.5f32, 0.0f32);
    for dy in -1..=1 {
        for dx in -1..=1 {
            let (ix, iy) = (cell.x as i32 + dx, cell.y as i32 + dy);
            let r = |k: i32| img::hash(ix + 7919 * seed, iy, k);
            let centre = Vec2::new((ix as f32 + r(1)) * cw, (iy as f32 + r(2)) * ch);
            let angle = (r(3) - 0.5) * 0.9;
            let (sa, ca) = angle.sin_cos();
            let d = p - centre;
            let local = Vec2::new(d.x * ca - d.y * sa, d.x * sa + d.y * ca);
            let (half_w, half_l) = (cw * (0.55 + 0.35 * r(4)), ch * (0.6 + 0.5 * r(5)));
            // A dab: square across with a ragged edge, tapering at its ends.
            let rag = (img::noise2(local.y / half_l * 3.0 + r(6) * 40.0, r(7) * 40.0) - 0.5) * 0.35;
            let across = (local.x / half_w).abs() + rag;
            let along = (local.y / half_l).abs();
            let cover = smoothstep(1.0, 0.85, across) * smoothstep(1.0, 0.75, along + 0.25 * (local.x / half_w).abs());
            // Later strokes (higher priority) lie on top.
            let priority = r(8) + cover;
            if cover > 0.35 && priority > best {
                best = priority;
                tint = 0.5 + (r(9) - 0.5) * 0.6;
                let bristle = (local.x / half_w * 9.0 + img::noise2(local.y / half_l * 2.0, r(10) * 30.0) * 3.0).sin();
                height = cover * (0.6 + 0.4 * bristle) * 2e-5;
            }
        }
    }
    (tint, height)
}

/// Modelling clay: soft lumps from pressing it into shape, and fingerprints here and there.
fn clay_height(t: &Texel) -> f32 {
    let p = t.p;
    let lumps = (img::fbm3(p * 110.0) - 0.5) * 6e-4 + (img::noise3(p * 260.0 + Vec3::splat(5.0)) - 0.5) * 8e-5;
    // Fingerprints: cells of about 2 cm, some with a print, each a set of rings around its
    // centre, warped, fading at its edge.
    let cell = 0.019f32;
    let c = (p / cell).floor();
    let mut print = 0.0f32;
    let mut weight = 0.0f32;
    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (ix, iy, iz) = (c.x as i32 + dx, c.y as i32 + dy, c.z as i32 + dz);
                let r = |k: i32| img::hash(ix * 31 + k, iy, iz);
                if r(1) < 0.45 {
                    continue;
                }
                let centre = (Vec3::new(ix as f32, iy as f32, iz as f32) + Vec3::new(r(2), r(3), r(4))) * cell;
                let d = p - centre;
                // Flatten along the surface normal so prints lie on the skin, elongated.
                let along = d - t.n * t.n.dot(d);
                let axis = Vec3::new(r(5) - 0.5, r(6) - 0.5, r(7) - 0.5).normalize_or(Vec3::X);
                let stretch = along.dot(axis) * 0.3;
                let dist = (along.length_squared() - stretch * stretch * (2.0 - 0.3)).max(0.0).sqrt();
                let radius = cell * (0.32 + 0.18 * r(8));
                let w = 1.0 - smoothstep(radius * 0.4, radius, dist);
                if w > weight {
                    weight = w;
                    let warp = (img::noise3(p * 400.0 + Vec3::splat(r(9) * 50.0)) - 0.5) * 0.6;
                    print = ((dist / 0.0009 + warp) * std::f32::consts::TAU).sin() * w * (0.5 + 0.5 * r(10));
                }
            }
        }
    }
    lumps + print * 1.4e-5
}
