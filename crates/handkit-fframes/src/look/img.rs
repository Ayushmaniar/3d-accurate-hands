//! Image building blocks for the looks: decoding the renderer's pixels, blurs, a distance
//! transform for ink lines, deterministic noise and the brush tile.

use std::sync::OnceLock;

use fframes_three_d::{Pixels, Vec3};
use rayon::prelude::*;

/// A rectangle of the frame in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x0: usize,
    pub y0: usize,
    pub w: usize,
    pub h: usize,
}

impl Rect {
    pub fn len(&self) -> usize {
        self.w * self.h
    }

    /// The box around the pixels with alpha, grown by `margin` and clipped to the frame;
    /// `None` when nothing is drawn.
    pub fn around(px: &Pixels, margin: usize) -> Option<Rect> {
        let (w, h) = (px.width as usize, px.height as usize);
        let rows: Vec<Option<(usize, usize)>> = px
            .rgba
            .par_chunks(w * 4)
            .map(|row| {
                let first = (0..w).find(|x| row[x * 4 + 3] > 0)?;
                let last = (0..w).rev().find(|x| row[x * 4 + 3] > 0)?;
                Some((first, last))
            })
            .collect();
        let ys: Vec<usize> = (0..h).filter(|y| rows[*y].is_some()).collect();
        let (y_lo, y_hi) = (*ys.first()?, *ys.last()?);
        let x_lo = rows.iter().flatten().map(|r| r.0).min()?;
        let x_hi = rows.iter().flatten().map(|r| r.1).max()?;
        let x0 = x_lo.saturating_sub(margin);
        let y0 = y_lo.saturating_sub(margin);
        let x1 = (x_hi + margin + 1).min(w);
        let y1 = (y_hi + margin + 1).min(h);
        Some(Rect { x0, y0, w: x1 - x0, h: y1 - y0 })
    }
}

/// sRGB 8 bit to linear.
pub fn srgb_to_linear_lut() -> &'static [f32; 256] {
    static LUT: OnceLock<[f32; 256]> = OnceLock::new();
    LUT.get_or_init(|| {
        std::array::from_fn(|i| {
            let c = i as f32 / 255.0;
            if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        })
    })
}

const ENC: usize = 4096;

/// Linear to sRGB in 0..1, through a table.
pub fn linear_to_srgb(c: f32) -> f32 {
    static LUT: OnceLock<Vec<f32>> = OnceLock::new();
    let lut = LUT.get_or_init(|| {
        (0..ENC)
            .map(|i| {
                let c = i as f32 / (ENC - 1) as f32;
                if c <= 0.0031308 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 }
            })
            .collect()
    });
    lut[((c.clamp(0.0, 1.0) * (ENC - 1) as f32) + 0.5) as usize]
}

pub fn srgb(c: [u8; 3]) -> Vec3 {
    let l = srgb_to_linear_lut();
    Vec3::new(l[c[0] as usize], l[c[1] as usize], l[c[2] as usize])
}

/// The pixels of `rect` as straight linear colour and alpha.
pub fn decode(px: &Pixels, rect: Rect) -> (Vec<Vec3>, Vec<f32>) {
    let lut = srgb_to_linear_lut();
    let fw = px.width as usize;
    let mut rgb = vec![Vec3::ZERO; rect.len()];
    let mut alpha = vec![0f32; rect.len()];
    rgb.par_chunks_mut(rect.w).zip(alpha.par_chunks_mut(rect.w)).enumerate().for_each(|(y, (rgb, alpha))| {
        let row = &px.rgba[((rect.y0 + y) * fw + rect.x0) * 4..];
        for x in 0..rect.w {
            let [r, g, b, a] = [row[x * 4], row[x * 4 + 1], row[x * 4 + 2], row[x * 4 + 3]];
            if a == 0 {
                continue;
            }
            let un = |c: u8| ((u32::from(c) * 255 + u32::from(a) / 2) / u32::from(a)).min(255) as usize;
            rgb[x] = Vec3::new(lut[un(r)], lut[un(g)], lut[un(b)]);
            alpha[x] = f32::from(a) / 255.0;
        }
    });
    (rgb, alpha)
}

/// A layer of premultiplied linear RGBA over `rect`.
pub struct Layer {
    pub rect: Rect,
    pub rgba: Vec<[f32; 4]>,
}

impl Layer {
    /// Adds `weight` times this layer into a frame-sized buffer.
    pub fn accumulate(&self, frame: &mut [[f32; 4]], frame_w: usize, weight: f32) {
        frame.par_chunks_mut(frame_w).skip(self.rect.y0).take(self.rect.h).enumerate().for_each(|(y, row)| {
            let src = &self.rgba[y * self.rect.w..(y + 1) * self.rect.w];
            for (d, s) in row[self.rect.x0..self.rect.x0 + self.rect.w].iter_mut().zip(src) {
                for i in 0..4 {
                    d[i] += s[i] * weight;
                }
            }
        });
    }
}

/// Encodes premultiplied linear RGBA over `rect` (`at(x, y)` in rect coordinates) as the
/// renderer's premultiplied sRGB pixels of a `width` x `height` frame.
pub fn encode(width: u32, height: u32, rect: Rect, at: impl Fn(usize, usize) -> [f32; 4] + Sync) -> Pixels {
    let fw = width as usize;
    let mut out = vec![0u8; fw * height as usize * 4];
    out.par_chunks_mut(fw * 4).skip(rect.y0).take(rect.h).enumerate().for_each(|(y, row)| {
        for x in 0..rect.w {
            let p = at(x, y);
            let a = p[3].clamp(0.0, 1.0);
            if a <= 0.0 {
                continue;
            }
            let a8 = (a * 255.0 + 0.5) as u8;
            let af = f32::from(a8) / 255.0;
            let o = &mut row[(rect.x0 + x) * 4..(rect.x0 + x) * 4 + 4];
            for i in 0..3 {
                o[i] = ((linear_to_srgb(p[i] / a) * af).clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            }
            o[3] = a8;
        }
    });
    Pixels { width, height, rgba: out }
}

/// Blurs a single channel image in place with three box blurs (close to a gaussian of
/// standard deviation `sigma`).
pub fn blur(buf: &mut [f32], w: usize, h: usize, sigma: f32) {
    if sigma < 0.3 {
        return;
    }
    // Box radius for three passes approximating the gaussian.
    let r = (((12.0 * sigma * sigma / 3.0 + 1.0).sqrt() - 1.0) / 2.0).round().max(1.0) as usize;
    for _ in 0..3 {
        buf.par_chunks_mut(w).for_each(|row| box_line(row, r));
    }
    let mut t = transpose(buf, w, h);
    for _ in 0..3 {
        t.par_chunks_mut(h).for_each(|col| box_line(col, r));
    }
    buf.copy_from_slice(&transpose(&t, h, w));
}

fn box_line(line: &mut [f32], r: usize) {
    let n = line.len();
    if n == 0 {
        return;
    }
    let src = line.to_vec();
    let at = |i: isize| src[i.clamp(0, n as isize - 1) as usize];
    let mut sum: f32 = (-(r as isize)..=r as isize).map(at).sum();
    let k = 1.0 / (2 * r + 1) as f32;
    for i in 0..n {
        line[i] = sum * k;
        sum += at(i as isize + r as isize + 1) - at(i as isize - r as isize);
    }
}

pub fn transpose(buf: &[f32], w: usize, h: usize) -> Vec<f32> {
    let mut out = vec![0f32; w * h];
    out.par_chunks_mut(h).enumerate().for_each(|(x, col)| {
        for (y, c) in col.iter_mut().enumerate() {
            *c = buf[y * w + x];
        }
    });
    out
}

/// The distance in pixels from each pixel to the nearest seed (exact euclidean, Felzenszwalb
/// and Huttenlocher). Without seeds every distance is large.
pub fn distance(seeds: &[bool], w: usize, h: usize) -> Vec<f32> {
    const FAR: f32 = 1e12;
    let mut d: Vec<f32> = seeds.iter().map(|s| if *s { 0.0 } else { FAR }).collect();
    d.par_chunks_mut(w).for_each(edt_line);
    let mut t = transpose(&d, w, h);
    t.par_chunks_mut(h).for_each(edt_line);
    let d = transpose(&t, h, w);
    d.into_par_iter().map(f32::sqrt).collect()
}

/// The 1D squared distance transform of `f` in place (seeds 0, the rest very large).
fn edt_line(f: &mut [f32]) {
    let n = f.len();
    if n == 0 || f.iter().all(|x| *x >= 1e11) {
        return;
    }
    let src = f.to_vec();
    let mut v = vec![0usize; n];
    let mut z = vec![0f32; n + 1];
    let mut k: isize = -1;
    for q in 0..n {
        if src[q] >= 1e11 {
            continue;
        }
        loop {
            if k < 0 {
                k = 0;
                v[0] = q;
                z[0] = f32::NEG_INFINITY;
                z[1] = f32::INFINITY;
                break;
            }
            let p = v[k as usize];
            let s = ((src[q] + (q * q) as f32) - (src[p] + (p * p) as f32)) / (2.0 * (q as f32 - p as f32));
            if s <= z[k as usize] {
                k -= 1;
                continue;
            }
            k += 1;
            v[k as usize] = q;
            z[k as usize] = s;
            z[k as usize + 1] = f32::INFINITY;
            break;
        }
    }
    let mut k = 0usize;
    for (q, out) in f.iter_mut().enumerate() {
        while z[k + 1] < q as f32 {
            k += 1;
        }
        let p = v[k];
        let dq = q as f32 - p as f32;
        *out = dq * dq + src[p];
    }
}

pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A hash of integer coordinates in 0..1.
pub fn hash(x: i32, y: i32, z: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343) ^ (y as u32).wrapping_mul(0xd816_3841) ^ (z as u32).wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^= h >> 15;
    (h & 0x00ff_ffff) as f32 / 16_777_216.0
}

fn fade(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Smooth value noise in 0..1.
pub fn noise2(x: f32, y: f32) -> f32 {
    let (xi, yi) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (fade(x - x.floor()), fade(y - y.floor()));
    let a = hash(xi, yi, 7) + (hash(xi + 1, yi, 7) - hash(xi, yi, 7)) * fx;
    let b = hash(xi, yi + 1, 7) + (hash(xi + 1, yi + 1, 7) - hash(xi, yi + 1, 7)) * fx;
    a + (b - a) * fy
}

/// Smooth 3D value noise in 0..1.
pub fn noise3(p: Vec3) -> f32 {
    let i = p.floor();
    let f = p - i;
    let (fx, fy, fz) = (fade(f.x), fade(f.y), fade(f.z));
    let (xi, yi, zi) = (i.x as i32, i.y as i32, i.z as i32);
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let c = |dx: i32, dy: i32, dz: i32| hash(xi + dx, yi + dy, zi + dz);
    let x00 = lerp(c(0, 0, 0), c(1, 0, 0), fx);
    let x10 = lerp(c(0, 1, 0), c(1, 1, 0), fx);
    let x01 = lerp(c(0, 0, 1), c(1, 0, 1), fx);
    let x11 = lerp(c(0, 1, 1), c(1, 1, 1), fx);
    lerp(lerp(x00, x10, fy), lerp(x01, x11, fy), fz)
}

/// Two octaves of [`noise3`].
pub fn fbm3(p: Vec3) -> f32 {
    (noise3(p) * 0.65 + noise3(p * 2.03 + Vec3::splat(17.3)) * 0.35).clamp(0.0, 1.0)
}

/// A wrapping colour tile sampled bilinearly, in linear RGB.
pub struct Tile {
    pub size: usize,
    pub rgb: Vec<Vec3>,
    pub mean: Vec3,
}

impl Tile {
    pub fn decode(bytes: &[u8], size: usize) -> Option<Tile> {
        let tex = fframes_three_d::Texture::decode(bytes).ok()?;
        let rgb: Vec<Vec3> = (0..size * size)
            .map(|i| {
                let (x, y) = (i % size, i / size);
                let uv = fframes_three_d::Vec2::new((x as f32 + 0.5) / size as f32, (y as f32 + 0.5) / size as f32);
                tex.sample(uv).truncate()
            })
            .collect();
        let mean = rgb.iter().copied().sum::<Vec3>() / rgb.len() as f32;
        Some(Tile { size, rgb, mean })
    }

    /// The colour at `u`, `v` in tiles (it repeats).
    pub fn sample(&self, u: f32, v: f32) -> Vec3 {
        let n = self.size as f32;
        let (x, y) = (u.rem_euclid(1.0) * n - 0.5, v.rem_euclid(1.0) * n - 0.5);
        let (xi, yi) = (x.floor(), y.floor());
        let (fx, fy) = (x - xi, y - yi);
        let s = self.size as i32;
        let at = |dx: i32, dy: i32| self.rgb[((yi as i32 + dy).rem_euclid(s) * s + (xi as i32 + dx).rem_euclid(s)) as usize];
        let a = at(0, 0).lerp(at(1, 0), fx);
        let b = at(0, 1).lerp(at(1, 1), fx);
        a.lerp(b, fy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_is_euclidean() {
        let (w, h) = (9, 7);
        let mut seeds = vec![false; w * h];
        seeds[3 * w + 4] = true;
        let d = distance(&seeds, w, h);
        assert_eq!(d[3 * w + 4], 0.0);
        assert!((d[3 * w + 7] - 3.0).abs() < 1e-5);
        assert!((d[0] - (16.0f32 + 9.0).sqrt()).abs() < 1e-4);
        let none = distance(&vec![false; w * h], w, h);
        assert!(none.iter().all(|x| *x > 1e5));
    }

    #[test]
    fn blur_keeps_the_mean() {
        let (w, h) = (40, 30);
        let mut b: Vec<f32> = (0..w * h).map(|i| if i == 15 * w + 20 { 100.0 } else { 0.0 }).collect();
        blur(&mut b, w, h, 3.0);
        let sum: f32 = b.iter().sum();
        assert!((sum - 100.0).abs() < 1.0, "{sum}");
        assert!(b[15 * w + 20] < 10.0);
    }
}
