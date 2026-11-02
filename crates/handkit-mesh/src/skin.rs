//! Skin maps baked for the hand's atlas (`uv`): base colour, normal, occlusion / roughness and
//! a tone-free mask map, embedded in the glTF and written to `assets/skin/` for looks.
//!
//! Every texel is computed from the bind-pose 3D point it covers (solid texturing), so the
//! two sides of a chart seam agree and seams do not show. The detail comes from small tiles
//! made from generated skin photos (`tools/skin/prepare_tiles.py`), sampled triplanar in 3D:
//! pores on the back of the hand, ridges and fine lines on the palm, wrinkles over the
//! knuckles (in each joint's frame, so they run across the finger). The rest is procedural from
//! the rig: redder knuckles, fingertips and palm pads, a lighter palm, faint veins on the back
//! of the hand and the inner wrist, the palm's and fingers' creases (from the shape's own crease
//! curves), hairs on the forearm, and the nails (a photo of a nail plate mapped onto each
//! plate: pink bed, pale lunula, white free edge).
//!
//! The mask map (R redness, G nail, B cavity / crease) and the palm mask (the occlusion /
//! roughness map's B) carry no skin tone. The
//! colour map is [`tone_base`] of [`BAKED_TONE`] times the fine detail (pores, mottling, veins,
//! hairs, the nail photo), so a look rebuilds the colour map for any tone, texel by texel, as
//! `baked * tone_base(tone) / tone_base(BAKED_TONE)` ([`tone_albedo`]): every tone keeps its
//! redness, creases, nails and palm (lighter on darker skin, as in real hands).

use glam::{DAffine3, DVec2, DVec3, dvec2, dvec3};
use handkit_core::rig::{Digit, HandGeom, Seg};

use crate::sdf::{Field, NailPlate};

/// The skin tone the baked colour map shows (sRGB): `medium`.
pub const BAKED_TONE: [u8; 3] = [0xd8, 0xa6, 0x8a];

/// sRGB bytes to linear RGB.
pub fn linear(c: [u8; 3]) -> [f32; 3] {
    c.map(|v| srgb_to_linear(v as f64 / 255.0) as f32)
}

fn lum(c: [f32; 3]) -> f32 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

fn mixf(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

fn mulf(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| a[i] * b[i])
}

/// How dark a tone is: 0 for light and medium skin, about 0.6 for brown, 1 for the darkest.
fn darkness(tone: [f32; 3]) -> f32 {
    smoothstep(0.30, 0.04, lum(tone) as f64) as f32
}

/// A tone's nail colour (linear): the skin lighter and pinker; on darker skin the nail bed
/// stays much lighter than the skin around it.
pub fn nail_color(tone: [f32; 3]) -> [f32; 3] {
    mixf(mulf(tone, [1.02, 0.9, 0.92]), linear([0xe0, 0xb8, 0xaa]), 0.42 + 0.18 * darkness(tone))
}

/// The skin's colour at a texel for a tone (linear RGB, no fine detail), from the mask map's
/// channels `m` = (redness, nail, cavity, palm), 0..1: the back of the hand a little darker and
/// less pink than `tone`, the palm pinker (and on darker skin much lighter), redder where R is
/// high, darker in creases, `nail` where G is 1.
pub fn tone_base(tone: [f32; 3], nail: [f32; 3], m: [f32; 4]) -> [f32; 3] {
    let dark = darkness(tone);
    let back = mulf(tone, [0.92, 0.95, 0.94]);
    let pink = mulf(tone, [0.95, 0.95, 1.07]);
    let pale = mixf(tone, linear([0xd6, 0x9e, 0x8a]), 0.55);
    let palm = mixf(pink, pale, 0.8 * dark);
    let skin = mixf(back, palm, m[3]);
    // Lit, dark skin reads more saturated than its albedo: take a little out.
    let skin = mixf(skin, [lum(skin); 3], 0.2 * dark);
    let skin = mixf(skin, mulf(skin, [1.0, 0.62, 0.68]), 0.35 * m[0] * (1.0 - 0.35 * dark));
    let skin = mulf(skin, mixf([1.0; 3], [0.78, 0.68, 0.66], m[2]));
    mixf(skin, nail, m[1])
}

/// The colour map for another tone, texel by texel: `baked` (linear) times
/// `tone_base(tone) / tone_base(BAKED_TONE)` at masks `m`.
pub fn tone_albedo(baked: [f32; 3], m: [f32; 4], tone: [f32; 3], nail: [f32; 3]) -> [f32; 3] {
    let from = baked_base(m);
    let to = tone_base(tone, nail, m);
    std::array::from_fn(|i| (baked[i] * to[i] / from[i].max(1e-4)).min(1.0))
}

fn baked_base(m: [f32; 4]) -> [f32; 3] {
    let t = linear(BAKED_TONE);
    tone_base(t, nail_color(t), m)
}

/// The encoded maps, ready for a glTF or for files.
#[derive(Clone)]
pub struct SkinMaps {
    /// Side of the colour and normal maps in texels (the others are half).
    pub size: u32,
    /// Base colour, sRGB, JPEG.
    pub albedo: Vec<u8>,
    /// Tangent-space normals (glTF convention: +Y toward the top of the image, i.e. -V), JPEG
    /// at high quality without chroma subsampling.
    pub normal: Vec<u8>,
    /// R ambient occlusion, G roughness, B palm skin (where glTF reads metalness, which the
    /// material's metallicFactor 0 turns off), JPEG: glTF's occlusion and metallic-roughness
    /// textures in one image.
    pub orm: Vec<u8>,
    /// R redness, G nail, B cavity; linear, JPEG.
    pub masks: Vec<u8>,
    /// Bake statistics: texels covered by the charts and texels covered twice.
    pub covered: f64,
    pub overlap: usize,
}

/// Per-vertex inputs (bind pose of the right hand, after the UV split).
pub struct Surface<'a> {
    pub positions: &'a [DVec3],
    pub normals: &'a [DVec3],
    pub uvs: &'a [[f32; 2]],
    pub indices: &'a [u32],
    /// (palm side 0..1, weight on the digits 0..1, ambient occlusion 0..1, crease depth in m).
    pub attrs: &'a [[f32; 4]],
}

// The detail tiles (see tools/skin/prepare_tiles.py).
static TILE_BACK: &[u8] = include_bytes!("../../../assets/skin/tiles/back.png");
static TILE_PALM: &[u8] = include_bytes!("../../../assets/skin/tiles/palm.png");
static TILE_KNUCKLE: &[u8] = include_bytes!("../../../assets/skin/tiles/knuckle.png");
static TILE_NAIL: &[u8] = include_bytes!("../../../assets/skin/tiles/nail.png");

/// A seamless tile: linear colour ratio to its local mean (1 = neutral) and height (in standard
/// deviations).
struct Tile {
    w: usize,
    h: usize,
    px: Vec<[f32; 4]>,
}

fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

fn linear_to_srgb(c: f64) -> f64 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.0031308 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 }
}

impl Tile {
    fn detail(bytes: &[u8]) -> Tile {
        let img = image::load_from_memory(bytes).expect("skin tile").to_rgba8();
        let (w, h) = (img.width() as usize, img.height() as usize);
        let px = img.pixels().map(|p| [p[0] as f32 / 255.0 + 0.5, p[1] as f32 / 255.0 + 0.5, p[2] as f32 / 255.0 + 0.5, (p[3] as f32 - 128.0) / 32.0]).collect();
        Tile { w, h, px }
    }

    /// An sRGB photo as linear colour (alpha 1).
    fn photo(bytes: &[u8]) -> Tile {
        let img = image::load_from_memory(bytes).expect("nail tile").to_rgb8();
        let (w, h) = (img.width() as usize, img.height() as usize);
        let px = img.pixels().map(|p| [0, 1, 2].map(|i| srgb_to_linear(p[i] as f64 / 255.0) as f32)).map(|c| [c[0], c[1], c[2], 1.0]).collect();
        Tile { w, h, px }
    }

    /// Bilinear, at (u, v) in tile units; wraps (or clamps).
    fn at(&self, u: f64, v: f64, wrap: bool) -> [f64; 4] {
        let (x, y) = (u * self.w as f64 - 0.5, v * self.h as f64 - 0.5);
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let idx = |i: f64, j: f64| {
            let (i, j) = (i as i64, j as i64);
            let (i, j) = if wrap {
                (i.rem_euclid(self.w as i64), j.rem_euclid(self.h as i64))
            } else {
                (i.clamp(0, self.w as i64 - 1), j.clamp(0, self.h as i64 - 1))
            };
            self.px[j as usize * self.w + i as usize]
        };
        let (a, b, c, d) = (idx(x0, y0), idx(x0 + 1.0, y0), idx(x0, y0 + 1.0), idx(x0 + 1.0, y0 + 1.0));
        std::array::from_fn(|k| {
            let top = a[k] as f64 * (1.0 - fx) + b[k] as f64 * fx;
            let bottom = c[k] as f64 * (1.0 - fx) + d[k] as f64 * fx;
            top * (1.0 - fy) + bottom * fy
        })
    }

    /// Triplanar in a frame: `q` and `n` local, `scale` meters per tile. Each plane's sample is
    /// shifted so the three do not line up.
    fn triplanar(&self, q: DVec3, n: DVec3, scale: f64) -> [f64; 4] {
        let w = n.abs().powf(4.0);
        let w = w / (w.x + w.y + w.z).max(1e-9);
        let s = q / scale;
        let mut out = [0.0; 4];
        for (wt, (u, v)) in [(w.x, (s.y + 0.37, s.z + 0.11)), (w.y, (s.x + 0.71, s.z + 0.53)), (w.z, (s.x, s.y))] {
            if wt < 0.01 {
                continue;
            }
            let t = self.at(u, v, true);
            for k in 0..4 {
                out[k] += t[k] * wt;
            }
        }
        out
    }
}

/// Value noise in 3D, 0..1.
/// A hash of three integers, 0..1.
fn hash(i: i64, j: i64, k: i64) -> f64 {
    let mut h = (i.wrapping_mul(73856093) ^ j.wrapping_mul(19349663) ^ k.wrapping_mul(83492791)) as u64;
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1e995);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2545f4914f6cdd1d);
    h ^= h >> 29;
    (h & 0xffff) as f64 / 65535.0
}

fn noise(p: DVec3) -> f64 {
    let f = p.floor();
    let t = p - f;
    let s = t * t * (DVec3::splat(3.0) - 2.0 * t);
    let (i, j, k) = (f.x as i64, f.y as i64, f.z as i64);
    let mut acc = 0.0;
    for c in 0..8 {
        let (dx, dy, dz) = (c & 1, (c >> 1) & 1, (c >> 2) & 1);
        let w = (if dx == 1 { s.x } else { 1.0 - s.x }) * (if dy == 1 { s.y } else { 1.0 - s.y }) * (if dz == 1 { s.z } else { 1.0 - s.z });
        acc += w * hash(i + dx, j + dy, k + dz);
    }
    acc
}

fn fbm(p: DVec3) -> f64 {
    (noise(p) * 0.5 + noise(p * 2.03 + 17.0) * 0.3 + noise(p * 4.1 + 31.0) * 0.2).clamp(0.0, 1.0)
}

fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mix3(a: DVec3, b: DVec3, t: f64) -> DVec3 {
    a + (b - a) * t
}

/// sRGB triple to linear.
fn lin(r: f64, g: f64, b: f64) -> DVec3 {
    dvec3(srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b))
}

fn seg_dist2(p: DVec2, a: DVec2, b: DVec2) -> f64 {
    let (pa, ba) = (p - a, b - a);
    let t = (pa.dot(ba) / ba.length_squared().max(1e-18)).clamp(0.0, 1.0);
    (pa - ba * t).length()
}

/// A joint whose back gets wrinkles and redness.
struct Knuckle {
    to_local: DAffine3,
    /// The finger's radius there.
    r: f64,
    wrinkle: f64,
    red: f64,
}

struct Vein {
    pts: Vec<DVec2>,
    width: f64,
    lo: DVec2,
    hi: DVec2,
}

impl Vein {
    /// A vein through `pts`, smoothed (Catmull-Rom), with a soft profile `width` wide.
    fn new(key: Vec<DVec2>, width: f64) -> Vein {
        let mut pts = Vec::new();
        let at = |i: i64| key[i.clamp(0, key.len() as i64 - 1) as usize];
        for i in 0..key.len() as i64 - 1 {
            let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
            for k in 0..8 {
                let t = k as f64 / 8.0;
                let (t2, t3) = (t * t, t * t * t);
                pts.push(0.5 * (2.0 * p1 + (p2 - p0) * t + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2 + (3.0 * p1 - p0 - 3.0 * p2 + p3) * t3));
            }
        }
        pts.push(*key.last().unwrap());
        let (mut lo, mut hi) = (DVec2::splat(f64::INFINITY), DVec2::splat(f64::NEG_INFINITY));
        for p in &pts {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
        Vein { pts, width, lo: lo - DVec2::splat(5.0 * width), hi: hi + DVec2::splat(5.0 * width) }
    }

    fn at(&self, p: DVec2) -> f64 {
        if p.x < self.lo.x || p.y < self.lo.y || p.x > self.hi.x || p.y > self.hi.y {
            return 0.0;
        }
        let d = self.pts.windows(2).map(|w| seg_dist2(p, w[0], w[1])).fold(f64::INFINITY, f64::min);
        // A soft, wide profile: a vein shows as a gentle swell under the skin, not a cord.
        (-(d / (1.7 * self.width)).powi(2)).exp()
    }
}

/// What the bake knows about the hand.
struct Anatomy<'a> {
    field: &'a Field,
    hand: DAffine3,
    knuckles: Vec<Knuckle>,
    /// Fingertips: (point, falloff).
    tips: Vec<(DVec3, f64)>,
    back_veins: Vec<Vein>,
    wrist_veins: Vec<Vein>,
    /// Pads on the palm: (hand-local center, radii).
    pads: Vec<(DVec3, DVec3)>,
    back: Tile,
    palm: Tile,
    knuckle: Tile,
    nail: Tile,
    /// The nail photo's longitudinal ridges: height per column, -1..1.
    ridges: Vec<f64>,
    /// Where the back veins fade out: (on the forearm, near the knuckles), hand-local y.
    vein_y: (f64, f64),
    /// The forearm's frame (world to local), for its hair.
    forearm: DAffine3,
}

/// Inputs interpolated at a texel.
#[derive(Clone, Copy)]
struct Attr {
    palmar: f64,
    digit: f64,
    ao: f64,
    crease: f64,
}

impl<'a> Anatomy<'a> {
    fn new(h: &HandGeom, field: &'a Field) -> Self {
        let bone = |name: &str| h.bones.iter().find(|b| b.name == name).unwrap_or_else(|| panic!("no bone {name}"));
        let radius = |d: Digit, s: Seg| h.capsules.iter().find(|c| c.digit == Some(d) && c.seg == s && c.part != "palm").map(|c| c.r).unwrap_or(0.008);
        let hand = bone("hand").frame.inverse();
        let mut knuckles = Vec::new();
        let mut tips = Vec::new();
        for d in Digit::ALL {
            let joints: [(&str, Seg, f64, f64); 3] = if d == Digit::Thumb {
                [("mcp", Seg::Proximal, 0.4, 0.45), ("ip", Seg::Proximal, 0.8, 0.6), ("", Seg::Distal, 0.0, 0.0)]
            } else {
                [("mcp", Seg::Proximal, 0.15, 0.6), ("pip", Seg::Proximal, 0.9, 0.8), ("dip", Seg::Middle, 0.7, 0.6)]
            };
            for (j, seg, wrinkle, red) in joints {
                if j.is_empty() {
                    continue;
                }
                let b = bone(&format!("{}_{j}", d.name()));
                let r = radius(d, seg) * if j == "mcp" && d != Digit::Thumb { 1.25 } else { 1.0 };
                knuckles.push(Knuckle { to_local: b.frame.inverse(), r, wrinkle, red });
            }
            let distal = bone(&format!("{}_{}", d.name(), if d == Digit::Thumb { "ip" } else { "dip" }));
            tips.push((distal.origin + distal.along.normalize() * distal.len, 0.75 * distal.len));
        }

        // Veins in the hand's frame (x, y), placed from the knuckles: f runs from the pinky's
        // knuckle (-1) to the index's (+1), y in fractions of the knuckles' height above the
        // wrist (negative on the forearm).
        let m = |d: &str| hand.transform_point3(bone(&format!("{d}_mcp")).origin);
        let (mi, mp) = (m("index"), m("pinky"));
        let ky = (mi.y + m("middle").y + m("ring").y + mp.y) / 4.0;
        let at = |f: f64, y: f64| dvec2((mi.x + mp.x) / 2.0 + f * (mi.x - mp.x) / 2.0, y * ky);
        let line = |pts: &[(f64, f64)], w: f64| Vein::new(pts.iter().map(|&(f, y)| at(f, y)).collect(), w);
        // The dorsal network: from the gaps between the knuckles down and toward the thumb side
        // of the wrist, joined by arches, with a vein up each edge of the hand.
        let back_veins = vec![
            line(&[(0.35, 0.82), (0.32, 0.62), (0.45, 0.42), (0.68, 0.22), (0.84, 0.02), (0.9, -0.3)], 0.0017),
            line(&[(-0.32, 0.82), (-0.22, 0.6), (-0.02, 0.44), (0.25, 0.33), (0.58, 0.29)], 0.0014),
            line(&[(-0.75, 0.78), (-0.7, 0.55), (-0.5, 0.38), (-0.25, 0.24), (-0.15, 0.04), (-0.2, -0.3)], 0.0015),
            line(&[(-0.02, 0.44), (-0.12, 0.33), (-0.25, 0.24)], 0.0011),
            line(&[(0.86, 0.72), (1.0, 0.46), (0.97, 0.22), (0.86, 0.04)], 0.0013),
            line(&[(-0.88, 0.56), (-1.0, 0.3), (-0.97, 0.0), (-0.92, -0.3)], 0.0013),
        ];
        let wrist_veins = vec![
            line(&[(0.3, 0.02), (0.24, -0.3), (0.3, -0.62)], 0.0011),
            line(&[(-0.2, -0.04), (-0.14, -0.34), (-0.24, -0.62)], 0.0011),
            line(&[(0.25, -0.24), (0.05, -0.38), (-0.15, -0.42)], 0.0008),
        ];
        let vein_y = (-0.45 * ky, 0.85 * ky);
        let pads = vec![
            // Thenar and hypothenar, as in the shape (sdf.rs), and the pads under the knuckles.
            (dvec3(0.02, 0.03, 0.01), dvec3(0.017, 0.026, 0.02)),
            (dvec3(-0.027, 0.034, 0.01), dvec3(0.014, 0.028, 0.02)),
            (dvec3(mi.x * 0.5 + mp.x * 0.5, ky - 0.006, 0.01), dvec3((mi.x - mp.x).abs() * 0.75, 0.009, 0.02)),
        ];

        let nail = Tile::photo(TILE_NAIL);
        // The ridges: each column's luminance against its neighbours'.
        let cols: Vec<f64> = (0..nail.w)
            .map(|i| (nail.h / 5..nail.h * 4 / 5).map(|j| nail.px[j * nail.w + i][1] as f64).sum::<f64>() / (nail.h * 3 / 5) as f64)
            .collect();
        let mut ridges: Vec<f64> = (0..cols.len())
            .map(|i| {
                let lo = i.saturating_sub(6);
                let hi = (i + 7).min(cols.len());
                cols[i] - cols[lo..hi].iter().sum::<f64>() / (hi - lo) as f64
            })
            .collect();
        let peak = ridges.iter().fold(1e-9_f64, |a, b| a.max(b.abs()));
        ridges.iter_mut().for_each(|r| *r /= peak);

        Anatomy {
            field,
            hand,
            knuckles,
            tips,
            back_veins,
            wrist_veins,
            pads,
            back: Tile::detail(TILE_BACK),
            palm: Tile::detail(TILE_PALM),
            knuckle: Tile::detail(TILE_KNUCKLE),
            nail,
            ridges,
            vein_y,
            forearm: bone("forearm").frame.inverse(),
        }
    }

    /// The back of a joint: 1 over the knuckle, fading along the finger over `reach` radii and
    /// around its sides.
    fn knuckle_mask(k: &Knuckle, q: DVec3, reach: f64) -> f64 {
        let along = (-(q.y / (reach * k.r)).powi(2)).exp();
        let across = smoothstep(1.6 * k.r, 0.9 * k.r, q.x.abs());
        along * across * smoothstep(-0.1, 0.7, -q.z / k.r)
    }

    /// The crease lines: 0 off them, 1 at the bottom of a groove.
    fn crease_line(&self, p: DVec3) -> f64 {
        let mut best: f64 = 0.0;
        for cut in &self.field.cuts {
            if (p - cut.bound.0).length() > cut.bound.1 + 0.001 {
                continue;
            }
            let d = cut.segs.iter().map(|(a, b)| {
                let (pa, ba) = (p - *a, *b - *a);
                let t = (pa.dot(ba) / ba.length_squared().max(1e-18)).clamp(0.0, 1.0);
                (pa - ba * t).length()
            });
            let d = d.fold(f64::INFINITY, f64::min) - cut.r;
            best = best.max((-d / 0.00014).clamp(0.0, 1.0).powi(3));
        }
        best
    }

    /// The nail plate a point is on: (mask, across -1..1, along 0 root .. 1 free edge).
    fn nail_at(&self, p: DVec3) -> (f64, f64, f64) {
        let mut best = (0.0, 0.0, 0.0);
        for n in &self.field.nails {
            let m = n.mask(p);
            if m > best.0 {
                best = (m, plate_coords(n, p).0, plate_coords(n, p).1);
            }
        }
        best
    }


    /// (veins on the back, veins on the inner wrist), 0..1.
    fn veins(&self, q: DVec3, palmar: f64) -> (f64, f64) {
        let p = dvec2(q.x, q.y);
        let span = smoothstep(self.vein_y.0, self.vein_y.0 + 0.02, q.y) * smoothstep(self.vein_y.1, self.vein_y.1 - 0.012, q.y);
        let back = self.back_veins.iter().map(|v| v.at(p)).fold(0.0, f64::max) * smoothstep(0.55, 0.2, palmar) * span;
        let wrist = self.wrist_veins.iter().map(|v| v.at(p)).fold(0.0, f64::max) * smoothstep(0.5, 0.85, palmar) * smoothstep(0.004, -0.01, q.y) * smoothstep(self.vein_y.0 - 0.02, self.vein_y.0 + 0.01, q.y);
        (back, wrist)
    }

    /// Hairs on the forearm, 0..1: short strokes scattered on a grid around the forearm (u
    /// around it, v along it), denser on its back and away from the wrist.
    fn hair(&self, p: DVec3, hand_y: f64, palmar: f64) -> f64 {
        let density = smoothstep(-0.01, -0.06, hand_y) * (0.3 + 0.7 * smoothstep(0.7, 0.3, palmar));
        if density <= 0.0 {
            return 0.0;
        }
        let f = self.forearm.transform_point3(p);
        let at = dvec2(f.z.atan2(f.x) * 0.03, f.y);
        let cell = 0.0015;
        let (ci, cj) = ((at.x / cell).floor() as i64, (at.y / cell).floor() as i64);
        let mut best: f64 = 0.0;
        for i in ci - 2..=ci + 2 {
            for j in cj - 3..=cj + 2 {
                if hash(i, j, 1) > 0.8 * density {
                    continue;
                }
                let root = dvec2((i as f64 + hash(i, j, 2)) * cell, (j as f64 + hash(i, j, 3)) * cell);
                let a = 0.35 + 0.5 * (hash(i, j, 4) - 0.5);
                let len = 0.0025 + 0.002 * hash(i, j, 5);
                let tip = root + dvec2(a.sin(), -a.cos()) * len;
                let d = seg_dist2(at, root, tip);
                // Thinner toward the tip.
                let along = ((at - root).dot(tip - root) / (len * len)).clamp(0.0, 1.0);
                let w = 0.00012 * (1.0 - 0.5 * along);
                best = best.max((-(d / w).powi(2)).exp() * (0.6 + 0.4 * hash(i, j, 6)));
            }
        }
        best
    }

    /// How much of a point takes the palm's skin (palm and the palm side of the digits, not the
    /// forearm).
    fn palm_skin(q: DVec3, palmar: f64) -> f64 {
        smoothstep(0.25, 0.75, palmar) * smoothstep(-0.03, 0.008, q.y)
    }

    /// Height of the fine relief above the mesh, meters.
    fn height(&self, p: DVec3, n: DVec3, a: &Attr) -> f64 {
        let q = self.hand.transform_point3(p);
        let nq = self.hand.transform_vector3(n);
        let palm_w = Self::palm_skin(q, a.palmar);
        let back = self.back.triplanar(q, nq, 0.036)[3];
        let palm = if palm_w > 0.0 { self.palm.triplanar(q, nq, 0.036)[3] } else { 0.0 };
        let mut h = back * 0.011e-3 * (1.0 - palm_w) + palm * 0.02e-3 * palm_w;
        for k in &self.knuckles {
            let kq = k.to_local.transform_point3(p);
            if kq.length() > 2.5 * k.r {
                continue;
            }
            let m = Self::knuckle_mask(k, kq, 0.5) * k.wrinkle;
            if m > 0.01 {
                let t = self.knuckle.at(kq.x / 0.016 + 0.5, kq.y / 0.016 + 0.5, true)[3];
                h = h * (1.0 - 0.6 * m) + m * 0.035e-3 * t;
            }
        }
        h -= 0.03e-3 * self.crease_line(p);
        let (back_vein, wrist_vein) = self.veins(q, a.palmar);
        h += 0.11e-3 * back_vein + 0.05e-3 * wrist_vein;
        let (nm, s, t) = self.nail_at(p);
        if nm > 0.0 {
            let col = ((s * 0.5 + 0.5) * (self.ridges.len() - 1) as f64).clamp(0.0, (self.ridges.len() - 1) as f64);
            let ridge = self.ridges[col as usize];
            h = h * (1.0 - nm) + nm * (0.004e-3 * ridge * smoothstep(0.0, 0.2, t));
        }
        h
    }

    /// (linear albedo, roughness, occlusion, masks).
    fn shade(&self, p: DVec3, n: DVec3, a: &Attr) -> (DVec3, f64, f64, [f64; 4]) {
        let q = self.hand.transform_point3(p);
        let nq = self.hand.transform_vector3(n);
        let palmar = smoothstep(0.25, 0.75, a.palmar);
        let dorsal = 1.0 - palmar;
        let forearm = smoothstep(0.0, -0.03, q.y);
        let palm_w = Self::palm_skin(q, a.palmar);
        let back = self.back.triplanar(q, nq, 0.036);
        let palm = self.palm.triplanar(q, nq, 0.036);
        let mut ratio = DVec3::from_slice(&back[..3]) * (1.0 - palm_w) + DVec3::from_slice(&palm[..3]) * palm_w;

        // Redness: knuckles, fingertips, palm pads, and the palm side of the hand and fingers.
        let mut knuckle_red: f64 = 0.0;
        let mut wrinkle: f64 = 0.0;
        let mut groove: f64 = 0.0;
        for k in &self.knuckles {
            let kq = k.to_local.transform_point3(p);
            if kq.length() > 2.5 * k.r {
                continue;
            }
            knuckle_red = knuckle_red.max(Self::knuckle_mask(k, kq, 0.9) * k.red);
            let m = Self::knuckle_mask(k, kq, 0.5);
            if m * k.wrinkle > 0.01 {
                let t = self.knuckle.at(kq.x / 0.016 + 0.5, kq.y / 0.016 + 0.5, true);
                let w = m * k.wrinkle;
                ratio = ratio * (1.0 - w) + DVec3::from_slice(&t[..3]) * w;
                wrinkle = wrinkle.max(w);
                groove = groove.max(w * (-t[3] * 0.5).clamp(0.0, 1.0));
            }
        }
        let tip: f64 = self.tips.iter().map(|(c, s)| (-((p - *c).length() / s).powi(2)).exp()).fold(0.0, f64::max);
        let pad: f64 = self.pads.iter().map(|(c, r)| (-((q - *c) / *r).length_squared()).exp()).fold(0.0, f64::max) * palmar * (1.0 - a.digit);
        let crease = self.crease_line(p);
        let cavity = (crease + (a.crease / 0.0004).clamp(0.0, 1.0) * 0.9 + groove * 0.6).min(1.0).max((1.0 - a.ao) * 0.35);
        let mottle = fbm(p / 0.006);
        let red_noise = fbm(p / 0.011 + 5.0);
        let redness = (0.8 * knuckle_red + 0.75 * tip * (0.6 + 0.4 * palmar) + 0.4 * pad + 0.15 * palm_w * a.digit + 0.12 * palm_w + 0.35 * crease + 0.3 * (red_noise - 0.5))
            .clamp(0.0, 1.0);

        // The base colour of the baked tone (redness, creases, palm and nails from the masks),
        // then the fine detail, which every tone keeps (`tone_albedo`).
        let (nm, s, t) = self.nail_at(p);
        let masks = [redness, nm, cavity, palm_w];
        let base = DVec3::from_array(baked_base(masks.map(|v| v as f32)).map(f64::from));
        // The back of the forearm is a little darker, its inner side a little lighter.
        let mut detail = DVec3::splat(1.0 - 0.07 * forearm * dorsal + 0.03 * forearm * palmar);
        // The tiles' fine variation, mostly in lightness.
        let lr = ratio.dot(dvec3(0.2126, 0.7152, 0.0722));
        let tiles = DVec3::splat(lr) + (ratio - DVec3::splat(lr)) * 0.35;
        detail *= DVec3::ONE + (tiles - DVec3::ONE) * 0.8;
        detail *= 1.0 + 0.14 * (mottle - 0.5);
        // Veins: a slight blue-green shift where they run under thin skin.
        let (vein_back, vein_wrist) = self.veins(q, a.palmar);
        let vein = (vein_back * 0.6 + vein_wrist * 0.7).min(1.0);
        detail *= mix3(DVec3::ONE, dvec3(0.93, 0.975, 1.0), vein);
        // Fine dark hairs on the forearm, lying toward the elbow.
        let hair = self.hair(p, q.y, a.palmar);
        detail = mix3(detail, lin(0.2, 0.14, 0.1) / base.max(DVec3::splat(1e-3)), 0.55 * hair);
        let mut c = base * detail;

        // Nails: the photo of a plate over the nail colour.
        let mut rough = 0.55 * dorsal + 0.62 * palmar + 0.05 * palm_w * a.digit + 0.08 * crease + 0.06 * wrinkle + 0.06 * (mottle - 0.5);
        if nm > 0.0 {
            let photo = self.nail.at(s * 0.5 + 0.5, 1.0 - t, false);
            // The photo's variation around its mean colour, over the tone's nail colour (the
            // base there).
            let plate = DVec3::from_slice(&photo[..3]) / dvec3(0.56, 0.33, 0.3) * 0.9;
            c = base * mix3(detail, plate, nm);
            rough = rough * (1.0 - nm) + 0.3 * nm;
        }
        let ao = a.ao * (1.0 - 0.3 * crease) * (1.0 - 0.25 * groove);
        (c, rough.clamp(0.2, 0.9), ao, masks)
    }
}

/// Plate coordinates: across -1..1, along 0 at the root to 1 at the free edge.
fn plate_coords(n: &NailPlate, p: DVec3) -> (f64, f64) {
    let q = n.ph.to_local.transform_point3(p);
    ((q.x / n.hx).clamp(-1.0, 1.0), ((q.y - n.y0) / (n.y1 - n.y0)).clamp(0.0, 1.0))
}

/// Bakes the maps at `size` (colour and normal; the others at half) with `gutter` texels of
/// dilation around the charts.
pub fn bake(geom: &HandGeom, field: &Field, s: &Surface, size: u32, gutter: u32, texels_per_meter: f64) -> SkinMaps {
    let n = size as usize;
    let tris = s.indices.len() / 3;
    // Rasterize: each texel center to (triangle, barycentrics).
    let mut tri = vec![u32::MAX; n * n];
    let mut bary = vec![[0f32; 2]; n * n];
    let mut overlap = 0;
    let uv = |v: u32| {
        let t = s.uvs[v as usize];
        dvec2(t[0] as f64 * n as f64 - 0.5, t[1] as f64 * n as f64 - 0.5)
    };
    let barycentric = |t: usize, p: DVec2| -> (f64, f64, f64) {
        let (a, b, c) = (uv(s.indices[3 * t]), uv(s.indices[3 * t + 1]), uv(s.indices[3 * t + 2]));
        let det = (b - a).perp_dot(c - a);
        if det.abs() < 1e-12 {
            return (-1.0, -1.0, -1.0);
        }
        let l1 = (p - a).perp_dot(c - a) / det;
        let l2 = (b - a).perp_dot(p - a) / det;
        (1.0 - l1 - l2, l1, l2)
    };
    for t in 0..tris {
        let (a, b, c) = (uv(s.indices[3 * t]), uv(s.indices[3 * t + 1]), uv(s.indices[3 * t + 2]));
        let lo = a.min(b).min(c).floor().max(DVec2::ZERO);
        let hi = a.max(b).max(c).ceil().min(DVec2::splat(n as f64 - 1.0));
        for y in lo.y as usize..=hi.y as usize {
            for x in lo.x as usize..=hi.x as usize {
                let (l0, l1, l2) = barycentric(t, dvec2(x as f64, y as f64));
                if l0 < -1e-7 || l1 < -1e-7 || l2 < -1e-7 {
                    continue;
                }
                let i = y * n + x;
                if tri[i] != u32::MAX && l0.min(l1).min(l2) > 1e-3 {
                    let o = tri[i] as usize;
                    let (m0, m1, m2) = barycentric(o, dvec2(x as f64, y as f64));
                    if m0.min(m1).min(m2) > 1e-3 {
                        overlap += 1;
                    }
                }
                tri[i] = t as u32;
                bary[i] = [l1 as f32, l2 as f32];
            }
        }
    }
    let covered = tri.iter().filter(|t| **t != u32::MAX).count() as f64 / (n * n) as f64;
    // Dilate: empty texels next to a chart take a neighbour's triangle, extrapolated.
    for _ in 0..gutter {
        let mut add = Vec::new();
        for y in 0..n {
            for x in 0..n {
                if tri[y * n + x] != u32::MAX {
                    continue;
                }
                let mut from = u32::MAX;
                for (dx, dy) in [(-1i64, 0i64), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                    let (xx, yy) = (x as i64 + dx, y as i64 + dy);
                    if xx >= 0 && yy >= 0 && (xx as usize) < n && (yy as usize) < n && tri[yy as usize * n + xx as usize] != u32::MAX {
                        from = tri[yy as usize * n + xx as usize];
                        break;
                    }
                }
                if from != u32::MAX {
                    let (_, l1, l2) = barycentric(from as usize, dvec2(x as f64, y as f64));
                    add.push((y * n + x, from, [l1 as f32, l2 as f32]));
                }
            }
        }
        for (i, t, b) in add {
            tri[i] = t;
            bary[i] = b;
        }
    }

    let anatomy = Anatomy::new(geom, field);
    // Central differences over most of a texel: the relief keeps what a texel can show and
    // drops what would only alias (and bloat the PNG).
    let eps = 0.8 / texels_per_meter;
    // Shade every row on every core.
    let rows: Vec<usize> = (0..n).collect();
    let shaded: Vec<Vec<([u8; 3], [u8; 3], [f32; 2], [f32; 4])>> = crate::par_map(&rows, |&y| {
        (0..n)
            .map(|x| {
                let i = y * n + x;
                let t = tri[i];
                if t == u32::MAX {
                    return ([200, 160, 140], [128, 128, 255], [1.0, 0.5], [0.0, 0.0, 0.0, 0.0]);
                }
                let t = t as usize;
                let [l1, l2] = bary[i].map(|v| v as f64);
                let l0 = 1.0 - l1 - l2;
                let vs = [0, 1, 2].map(|k| s.indices[3 * t + k] as usize);
                let p = s.positions[vs[0]] * l0 + s.positions[vs[1]] * l1 + s.positions[vs[2]] * l2;
                let nrm = (s.normals[vs[0]] * l0 + s.normals[vs[1]] * l1 + s.normals[vs[2]] * l2).normalize_or_zero();
                let at = |k: usize| s.attrs[vs[0]][k] as f64 * l0 + s.attrs[vs[1]][k] as f64 * l1 + s.attrs[vs[2]][k] as f64 * l2;
                let a = Attr { palmar: at(0).clamp(0.0, 1.0), digit: at(1).clamp(0.0, 1.0), ao: at(2).clamp(0.0, 1.0), crease: at(3).max(0.0) };
                let (albedo, rough, ao, masks) = anatomy.shade(p, nrm, &a);

                // The bent normal from the height's gradient along the surface.
                let (t1, t2) = nrm.any_orthonormal_pair();
                let h = |d: DVec3| anatomy.height(p + d * eps, nrm, &a);
                let g1 = (h(t1) - h(-t1)) / (2.0 * eps);
                let g2 = (h(t2) - h(-t2)) / (2.0 * eps);
                let bent = (nrm - t1 * g1 - t2 * g2).normalize();
                // Encode in the frame the renderers build from the UVs: T = dP/du, B = dP/dv
                // on the triangle, made perpendicular to the normal; n = T x - B y + N z.
                let (pa, pb, pc) = (s.positions[vs[0]], s.positions[vs[1]], s.positions[vs[2]]);
                let (ua, ub, uc) = (uv(s.indices[3 * t]), uv(s.indices[3 * t + 1]), uv(s.indices[3 * t + 2]));
                let (e1, e2, d1, d2) = (pb - pa, pc - pa, ub - ua, uc - ua);
                let det = d1.perp_dot(d2);
                let sign = if det < 0.0 { -1.0 } else { 1.0 };
                let dpdu = (e1 * d2.y - e2 * d1.y) * sign;
                let dpdv = (e2 * d1.x - e1 * d2.x) * sign;
                let tt = (dpdu - nrm * nrm.dot(dpdu)).normalize_or_zero();
                let bb = (dpdv - nrm * nrm.dot(dpdv)).normalize_or_zero();
                // Solve bent_t = tt * mx - bb * my in the tangent plane.
                let bt = bent - nrm * nrm.dot(bent);
                let (a11, a12, a22) = (tt.dot(tt), -tt.dot(bb), bb.dot(bb));
                let (r1, r2) = (bt.dot(tt), -bt.dot(bb));
                let dd = a11 * a22 - a12 * a12;
                let (mx, my) = if dd.abs() > 1e-9 { ((r1 * a22 - r2 * a12) / dd, (a11 * r2 - a12 * r1) / dd) } else { (0.0, 0.0) };
                let m = dvec3(mx, my, nrm.dot(bent).max(0.05)).normalize();
                let enc = |v: f64| ((v * 0.5 + 0.5) * 255.0).round().clamp(0.0, 255.0) as u8;
                let srgb = |v: f64| (linear_to_srgb(v) * 255.0).round() as u8;
                (
                    [srgb(albedo.x), srgb(albedo.y), srgb(albedo.z)],
                    [enc(m.x), enc(m.y), enc(m.z)],
                    [ao as f32, rough as f32],
                    masks.map(|v| v as f32),
                )
            })
            .collect()
    });

    let mut albedo = Vec::with_capacity(n * n * 3);
    let mut normal = Vec::with_capacity(n * n * 3);
    for row in &shaded {
        for (c, nm, _, _) in row {
            albedo.extend_from_slice(c);
            normal.extend_from_slice(nm);
        }
    }
    // Occlusion, roughness and masks at half size (2x2 averages).
    let h = n / 2;
    let mut orm = Vec::with_capacity(h * h * 3);
    let mut masks = Vec::with_capacity(h * h * 3);
    for y in 0..h {
        for x in 0..h {
            let px = [(0, 0), (1, 0), (0, 1), (1, 1)].map(|(dx, dy)| &shaded[2 * y + dy][2 * x + dx]);
            let avg = |f: &dyn Fn(&([u8; 3], [u8; 3], [f32; 2], [f32; 4])) -> f32| px.iter().map(|p| f(p)).sum::<f32>() / 4.0;
            let to8 = |v: f32| (v * 255.0).round().clamp(0.0, 255.0) as u8;
            // B is the palm mask: glTF reads metalness there, times metallicFactor 0.
            orm.extend_from_slice(&[to8(avg(&|p| p.2[0])), to8(avg(&|p| p.2[1])), to8(avg(&|p| p.3[3]))]);
            for k in 0..3 {
                masks.push(to8(avg(&|p| p.3[k])));
            }
        }
    }
    SkinMaps {
        size,
        albedo: encode_jpeg(&albedo, size, 90),
        normal: encode_jpeg(&normal, size, 93),
        orm: encode_jpeg(&orm, size / 2, 92),
        masks: encode_jpeg(&masks, size / 2, 92),
        covered,
        overlap,
    }
}

fn encode_jpeg(rgb: &[u8], size: u32, quality: u8) -> Vec<u8> {
    use image::ImageEncoder;
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality).write_image(rgb, size, size, image::ExtendedColorType::Rgb8).expect("jpeg");
    out
}
