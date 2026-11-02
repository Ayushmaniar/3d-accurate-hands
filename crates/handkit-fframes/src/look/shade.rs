//! The post-process of each look, over the passes of one moment (straight linear colour).

use std::f32::consts::TAU;

use fframes_three_d::{Vec2, Vec3};
use rayon::prelude::*;

use super::img::{self, Layer, Rect, Tile, smoothstep};
use super::{Look, Style};

pub(crate) struct Inputs<'a> {
    pub look: &'a Look,
    pub rect: Rect,
    /// Frame height / 1080: sizes given in pixels at 1080p scale by it.
    pub scale: f32,
    /// Pixels per meter at the hand.
    pub ppm: f32,
    /// The hand's pixel in rect coordinates: screen-space textures move with it.
    pub anchor: Vec2,
    pub alpha: Vec<f32>,
    /// The lit colour pass (realistic, clay).
    pub color: Vec<Vec3>,
    /// View-space normals: x right, y up, z toward the camera (stylized looks).
    pub normal: Vec<Vec3>,
    pub nail: Vec<f32>,
    /// Object-space noise in 0..1, stuck to the skin.
    pub noise: Vec<f32>,
    /// Distance from the camera in meters.
    pub depth: Vec<f32>,
    pub brush: Option<&'a Tile>,
}

/// The key light of the stylized looks in view space: up and to the left, in front.
fn key() -> Vec3 {
    Vec3::new(-0.72, 0.5, 0.48).normalize()
}

fn luminance(c: Vec3) -> f32 {
    c.dot(Vec3::new(0.2126, 0.7152, 0.0722))
}

/// Ink line distances: to the silhouette, and to interior edges (normal creases, depth
/// steps) when normals are given.
fn edges(inp: &Inputs) -> (Vec<f32>, Option<Vec<f32>>) {
    let Rect { w, h, .. } = inp.rect;
    let inside: Vec<bool> = inp.alpha.iter().map(|a| *a >= 0.5).collect();
    let at = |x: usize, y: usize| y * w + x;
    let mut sil = vec![false; w * h];
    let mut int = vec![false; w * h];
    let stylized = !inp.normal.is_empty();
    // Normal creases sharper than this angle and depth steps larger than this draw lines.
    let crease = 0.62f32;
    let step = 0.007f32;
    for y in 0..h {
        for x in 0..w {
            let i = at(x, y);
            for (nx, ny) in [(x + 1, y), (x, y + 1)] {
                if nx >= w || ny >= h {
                    continue;
                }
                let j = at(nx, ny);
                if inside[i] != inside[j] {
                    sil[i] = true;
                    sil[j] = true;
                } else if stylized && inside[i] {
                    let bend = inp.normal[i].dot(inp.normal[j]) < crease;
                    let jump = (inp.depth[i] - inp.depth[j]).abs() > step;
                    if bend || jump {
                        // The line goes on the nearer side, the edge of the front surface.
                        let k = if inp.depth[i] <= inp.depth[j] { i } else { j };
                        int[k] = true;
                    }
                }
            }
        }
    }
    let sil_d = img::distance(&sil, w, h);
    let int_d = stylized.then(|| img::distance(&int, w, h));
    (sil_d, int_d)
}

/// The coverage of a line of half width `r` at distance `d`.
fn cover(d: f32, r: f32) -> f32 {
    if r <= 0.0 {
        return 0.0;
    }
    1.0 - smoothstep(r - 0.75, r + 0.75, d)
}

/// The colours of a cel palette, linear.
struct Palette {
    deep: Vec3,
    shadow: Vec3,
    mid: Vec3,
    light: Vec3,
}

impl Palette {
    fn toon(c: Vec3) -> Palette {
        Palette {
            deep: c * Vec3::new(0.42, 0.26, 0.25),
            shadow: c * Vec3::new(0.62, 0.42, 0.40),
            mid: c,
            light: (c * 1.22 + Vec3::new(0.05, 0.035, 0.02)).min(Vec3::ONE),
        }
    }

    /// Warm lights, cool violet shadows.
    fn arcane(c: Vec3) -> Palette {
        let l = luminance(c);
        let cool = Vec3::new(0.30, 0.30, 0.46) * l * 1.6;
        Palette {
            deep: (c * Vec3::new(0.30, 0.22, 0.26)).lerp(cool * 0.45, 0.35),
            shadow: (c * Vec3::new(0.58, 0.44, 0.46)).lerp(cool * 0.75, 0.3),
            mid: c * Vec3::new(0.8, 0.7, 0.66),
            light: (c * Vec3::new(1.25, 1.08, 0.9) + Vec3::new(0.05, 0.025, 0.0)).min(Vec3::ONE),
        }
    }
}

/// Toon, spiderverse and arcane, from the normals and data passes.
pub(crate) fn stylized(inp: &Inputs) -> Layer {
    let look = inp.look;
    let Rect { w, h, .. } = inp.rect;
    let s = inp.scale;
    let (sil_d, int_d) = edges(inp);
    let int_d = int_d.unwrap();
    let skin = img::srgb(look.skin);
    let nail = img::srgb(look.nail_color());
    let style = look.style;
    let (ps, pn) = match style {
        Style::Arcane => (Palette::arcane(skin), Palette::arcane(nail)),
        _ => (Palette::toon(skin), Palette::toon(nail)),
    };
    let tex = look.texture.unwrap_or(1.0) as f32;
    let r = look.outline as f32 * 0.5 * s;
    let ink = match style {
        Style::Arcane => skin * Vec3::new(0.09, 0.045, 0.045),
        Style::Toon => Vec3::new(0.02, 0.012, 0.01),
        _ => Vec3::splat(0.004),
    };
    let halftone = match style {
        Style::Spiderverse => look.halftone.unwrap_or(8.0) as f32 * s,
        _ => 0.0,
    };
    let l = match style {
        // From the side, for planes of light and shadow.
        Style::Arcane => Vec3::new(-0.85, 0.42, 0.38).normalize(),
        _ => key(),
    };
    // Screen position in meters at the hand, for textures that follow it.
    let meters = |x: usize, y: usize| (Vec2::new(x as f32, y as f32) - inp.anchor) / inp.ppm.max(1.0);

    // Ink coverage first (spiderverse reuses it, offset, for the misregistration).
    let lines: Vec<f32> = (0..w * h)
        .into_par_iter()
        .map(|i| {
            let (x, y) = (i % w, i / w);
            match style {
                Style::Spiderverse => {
                    // Uneven: thicker in the shadows and where the noise says so.
                    let q = meters(x, y);
                    let wobble = img::noise2(q.x * 90.0 + 3.1, q.y * 90.0) * 0.8 + 0.6;
                    let n = inp.normal[i];
                    let shade = if inp.alpha[i] > 0.0 { (1.0 - n.dot(l)).clamp(0.0, 2.0) * 0.3 } else { 0.3 };
                    let rr = r * wobble * (0.85 + shade);
                    cover(sil_d[i], rr).max(cover(int_d[i], rr * 0.55))
                }
                Style::Arcane => {
                    let q = meters(x, y);
                    let wobble = img::noise2(q.x * 140.0, q.y * 140.0 + 7.0) * 0.7 + 0.5;
                    cover(sil_d[i], r * wobble).max(cover(int_d[i], r * 0.8 * wobble) * 0.85)
                }
                _ => cover(sil_d[i], r).max(cover(int_d[i], r * 0.6)),
            }
        })
        .collect();

    let misreg = match style {
        Style::Spiderverse => look.misregister.unwrap_or(3.0) as f32 * s,
        _ => 0.0,
    };
    let line_at = |x: isize, y: isize| -> f32 {
        if x < 0 || y < 0 || x >= w as isize || y >= h as isize {
            0.0
        } else {
            lines[y as usize * w + x as usize]
        }
    };

    let rgba: Vec<[f32; 4]> = (0..w * h)
        .into_par_iter()
        .map(|i| {
            let (x, y) = (i % w, i / w);
            let a = inp.alpha[i];
            let line = lines[i];
            let mut out = [0f32; 4];
            if a > 0.0 {
                let n = inp.normal[i];
                let mut nl = n.dot(l);
                let m = inp.nail[i];
                let col = match style {
                    Style::Arcane => {
                        let q = meters(x, y);
                        let tile = inp.brush.map(|t| {
                            // About 7 cm of skin per tile, turned so strokes run diagonally.
                            let (u, v) = (q.x * 0.8 + q.y * 0.6, -q.x * 0.6 + q.y * 0.8);
                            (t.sample(u / 0.09, v / 0.09), t.mean)
                        });
                        let rel = tile.map_or(Vec3::ONE, |(c, mean)| c / mean);
                        // Brush luminance breaks the band edges; object noise moves them.
                        nl += (luminance(rel) - 1.0) * 0.7 * tex + (inp.noise[i] - 0.5) * 0.2 * tex;
                        let soft = 0.09;
                        let t0 = smoothstep(-0.5 - soft, -0.5 + soft, nl);
                        let t1 = smoothstep(-0.1 - soft, -0.1 + soft, nl);
                        let t2 = smoothstep(0.42 - soft, 0.42 + soft, nl);
                        let band = |p: &Palette| p.deep.lerp(p.shadow, t0).lerp(p.mid, t1).lerp(p.light, t2);
                        let mut c = band(&ps).lerp(band(&pn), m);
                        // Brush colour: the tile's warm/cool variation.
                        c *= Vec3::ONE.lerp(rel, 0.5 * tex);
                        // Warm bounce on the shadow side's rim.
                        let rim = smoothstep(0.45, 0.95, 1.0 - n.z.max(0.0)) * (1.0 - t1);
                        c += Vec3::new(0.22, 0.09, 0.03) * rim * luminance(skin) * 1.4;
                        c
                    }
                    _ => {
                        let aa = 0.025;
                        let t1 = smoothstep(0.18 - aa, 0.18 + aa, nl);
                        let t2 = smoothstep(0.72 - aa, 0.72 + aa, nl);
                        let band = |p: &Palette| p.shadow.lerp(p.mid, t1).lerp(p.light, t2);
                        let mut c = band(&ps).lerp(band(&pn), m);
                        if halftone > 0.0 {
                            // Ben-Day dots on a 45 degree grid, larger where it is darker.
                            let q = (Vec2::new(x as f32, y as f32) - inp.anchor) / halftone;
                            let (u, v) = ((q.x + q.y) * std::f32::consts::FRAC_1_SQRT_2, (q.y - q.x) * std::f32::consts::FRAC_1_SQRT_2);
                            let d = Vec2::new(u - u.round(), v - v.round()).length() * halftone;
                            let dark = (1.0 - smoothstep(-0.2, 0.45, nl)).max((1.0 - t1) * 0.55);
                            let radius = halftone * 0.62 * dark.sqrt();
                            let dot = cover(d, radius) * 0.9;
                            c = c.lerp(ps.deep.lerp(pn.deep, m), dot);
                        }
                        c
                    }
                };
                for k in 0..3 {
                    out[k] = col[k] * a;
                }
                out[3] = a;
            }
            // Ink over the fill, out to the line's own edge.
            let la = line.max(0.0);
            for k in 0..3 {
                out[k] = out[k] * (1.0 - la) + ink[k] * la;
            }
            out[3] = out[3] * (1.0 - la) + la;
            if misreg > 0.0 {
                // Misregistered magenta and cyan plates, under the hand.
                let (dx, dy) = (misreg, misreg * 0.35);
                let gm = line_at((x as f32 - dx).round() as isize, (y as f32 - dy).round() as isize) * 0.85;
                let gc = line_at((x as f32 + dx * 0.8).round() as isize, (y as f32 + dy * 1.4).round() as isize) * 0.8;
                let magenta = Vec3::new(0.85, 0.02, 0.32);
                let cyan = Vec3::new(0.0, 0.55, 0.85);
                let ga = gm.max(gc);
                if ga > 0.0 {
                    let gcol = (magenta * gm + cyan * gc * (1.0 - gm)) / (gm + gc * (1.0 - gm)).max(1e-4);
                    let behind = 1.0 - out[3];
                    for k in 0..3 {
                        out[k] += gcol[k] * ga * behind;
                    }
                    out[3] += ga * behind;
                }
            }
            out
        })
        .collect();
    Layer { rect: inp.rect, rgba }
}

/// Silhouette ink for the lit looks when `outline` is set.
fn lit_lines(inp: &Inputs) -> Option<Vec<f32>> {
    let r = inp.look.outline as f32 * 0.5 * inp.scale;
    (r > 0.0).then(|| edges(inp).0.iter().map(|d| cover(*d, r)).collect())
}

fn finish(inp: &Inputs, col: impl Fn(usize) -> Vec3 + Sync, ink: Vec3) -> Layer {
    let lines = lit_lines(inp);
    let rgba = (0..inp.rect.len())
        .into_par_iter()
        .map(|i| {
            let a = inp.alpha[i];
            let c = if a > 0.0 { col(i) * a } else { Vec3::ZERO };
            let la = lines.as_ref().map_or(0.0, |l| l[i]);
            let c = c * (1.0 - la) + ink * la;
            [c.x, c.y, c.z, a * (1.0 - la) + la]
        })
        .collect();
    Layer { rect: inp.rect, rgba }
}

/// The lit render with screen-space subsurface scattering (a red-weighted blur inside the
/// hand's alpha, mixed in) and warmer shadows.
pub(crate) fn realistic(inp: &Inputs) -> Layer {
    let Rect { w, h, .. } = inp.rect;
    let mix = inp.look.subsurface.unwrap_or(0.5) as f32;
    // Scattering distances of skin per channel (meters), red the farthest.
    let reach = [0.0032f32, 0.0013, 0.0008];
    let blurred: Vec<Vec<f32>> = (0..3)
        .into_par_iter()
        .map(|k| {
            let sigma = (reach[k] * inp.ppm).clamp(0.0, 40.0 * inp.scale);
            let mut c: Vec<f32> = inp.color.iter().zip(&inp.alpha).map(|(c, a)| c[k] * a).collect();
            let mut a = inp.alpha.clone();
            img::blur(&mut c, w, h, sigma);
            img::blur(&mut a, w, h, sigma);
            c.iter().zip(&a).map(|(c, a)| if *a > 1e-3 { c / a } else { 0.0 }).collect()
        })
        .collect();
    let warm = Vec3::new(1.2, 0.92, 0.82);
    finish(
        inp,
        |i| {
            let c = inp.color[i];
            let b = Vec3::new(blurred[0][i], blurred[1][i], blurred[2][i]);
            let mut c = c.lerp(b, mix);
            let dark = 1.0 - smoothstep(0.0, 0.2, luminance(c));
            c *= Vec3::ONE.lerp(warm, dark * 0.85);
            c
        },
        Vec3::splat(0.01),
    )
}

/// A thumbprint's ridges at `q` (meters on the screen at the hand), in -1..1, 0 where there is
/// none.
fn thumbprints(q: Vec2, ridge: f32) -> f32 {
    let cell = 0.02;
    let c = (q / cell).floor();
    let (mut best, mut h) = (0.0, 0.0);
    for dy in -1..=1 {
        for dx in -1..=1 {
            let (ix, iy) = (c.x as i32 + dx, c.y as i32 + dy);
            if img::hash(ix, iy, 11) < 0.3 {
                continue;
            }
            let centre = Vec2::new(ix as f32 + img::hash(ix, iy, 12), iy as f32 + img::hash(ix, iy, 13)) * cell;
            let radius = cell * (0.38 + 0.25 * img::hash(ix, iy, 14));
            let (sa, ca) = (img::hash(ix, iy, 15) * TAU).sin_cos();
            let d = q - centre;
            let e = Vec2::new(d.x * ca + d.y * sa, (-d.x * sa + d.y * ca) * 1.35);
            let dist = e.length();
            let weight = (1.0 - smoothstep(radius * 0.15, radius, dist)) * (0.5 + 0.5 * img::hash(ix, iy, 17));
            if weight > best {
                best = weight;
                let warp = img::noise2(e.x / ridge * 0.35 + ix as f32 * 7.0, e.y / ridge * 0.35) * 2.5;
                h = ((dist / ridge + warp) * TAU + img::hash(ix, iy, 16) * 6.0).sin() * weight;
            }
        }
    }
    h
}

/// Matte clay: the lit render (lumpy normals come from `dress`) with thumbprints pressed in
/// and a fine grain.
pub(crate) fn clay(inp: &Inputs) -> Layer {
    let Rect { w, h, .. } = inp.rect;
    let tex = inp.look.texture.unwrap_or(1.0) as f32;
    let ridge = (0.0011f32).max(4.0 / inp.ppm.max(1.0));
    let field: Vec<f32> = (0..w * h)
        .into_par_iter()
        .map(|i| {
            if inp.alpha[i] <= 0.0 {
                return 0.0;
            }
            let q = (Vec2::new((i % w) as f32, (i / w) as f32) - inp.anchor) / inp.ppm.max(1.0);
            thumbprints(q, ridge)
        })
        .collect();
    let at = |x: usize, y: usize| field[y.min(h - 1) * w + x.min(w - 1)];
    // Light from the upper left on the screen (x right, y down).
    let light = Vec2::new(-0.6, -0.8);
    finish(
        inp,
        |i| {
            let (x, y) = (i % w, i / w);
            let gx = at(x + 1, y) - at(x.saturating_sub(1), y);
            let gy = at(x, y + 1) - at(x, y.saturating_sub(1));
            // A ridge facing the light is lighter, its far side darker.
            let emboss = -(gx * light.x + gy * light.y) * 0.11 * tex;
            let grain = (img::hash(x as i32, y as i32, 5) - 0.5) * 0.035 * tex;
            inp.color[i] * (1.0 + emboss + grain).max(0.0)
        },
        Vec3::splat(0.01),
    )
}
