//! The post-process of each look, over the passes of one moment (straight linear colour).

use fframes_three_d::{Vec2, Vec3};
use rayon::prelude::*;

use super::img::{self, Layer, Rect, smoothstep};
use super::{Look, Paint, Style};

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
    /// The cores of the main creases, 0..1.
    pub cavity: Vec<f32>,
    /// The data map's G, stuck to the skin: the palm (toon, spiderverse) or the brush strokes'
    /// warm / cool shift, 0.5 neutral (arcane).
    pub detail: Vec<f32>,
    /// Distance from the camera in meters.
    pub depth: Vec<f32>,
    /// The colours of the hands' people and sleeves; none: the look's.
    pub tones: &'a [Paint],
    /// Each pixel's index in `tones` (stylized looks with more than one tone; else all 0).
    pub tone: Vec<u8>,
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

    /// Cloth in toon and spiderverse: the colour itself, darker and lighter.
    fn cloth(c: Vec3) -> Palette {
        Palette { deep: c * 0.4, shadow: c * 0.6, mid: c, light: (c * 1.25 + Vec3::splat(0.03)).min(Vec3::ONE) }
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

/// A person's colours in a stylized look: the cel palettes of skin and nails, and the palm (a
/// sleeve's: its cloth's palette, no palm).
struct Tone {
    ps: Palette,
    pn: Palette,
    /// The palm's colour relative to the back (much lighter on darker skin), from the same
    /// model as the realistic skin.
    palm_ratio: Vec3,
    skin: Vec3,
}

impl Tone {
    fn new(style: Style, paint: &Paint) -> Tone {
        let Paint { skin: skin_rgb, nail: nail_rgb, cloth } = *paint;
        let (skin, nail) = (img::srgb(skin_rgb), img::srgb(nail_rgb));
        if cloth {
            let p = || if style == Style::Arcane { Palette::arcane(skin) } else { Palette::cloth(skin) };
            return Tone { ps: p(), pn: p(), palm_ratio: Vec3::ONE, skin };
        }
        let (ps, pn) = match style {
            Style::Arcane => (Palette::arcane(skin), Palette::arcane(nail)),
            _ => (Palette::toon(skin), Palette::toon(nail)),
        };
        let t = handkit_mesh::skin::linear(skin_rgb);
        let back = handkit_mesh::skin::tone_base(t, t, [0.0; 4]);
        let palm = handkit_mesh::skin::tone_base(t, t, [0.0, 0.0, 0.0, 1.0]);
        let palm_ratio = Vec3::new(palm[0] / back[0].max(1e-4), palm[1] / back[1].max(1e-4), palm[2] / back[2].max(1e-4));
        Tone { ps, pn, palm_ratio, skin }
    }
}

/// Toon, spiderverse and arcane, from the normals and data passes.
pub(crate) fn stylized(inp: &Inputs) -> Layer {
    let look = inp.look;
    let Rect { w, h, .. } = inp.rect;
    let s = inp.scale;
    let (sil_d, int_d) = edges(inp);
    let int_d = int_d.unwrap();
    let style = look.style;
    let own = [Paint { skin: look.skin, nail: look.nail_color(), cloth: false }];
    let tones = if inp.tones.is_empty() { &own[..] } else { inp.tones };
    let tones: Vec<Tone> = tones.iter().map(|p| Tone::new(style, p)).collect();
    let tone_at = |i: usize| &tones[inp.tone.get(i).map_or(0, |t| *t as usize).min(tones.len() - 1)];
    // Crease ink: the main creases only (palm lines, finger joints), thin.
    let crease_ink = match style {
        Style::Toon => 0.7,
        Style::Spiderverse => 0.85,
        Style::Arcane => 0.75,
        _ => 0.0,
    };
    let tex = look.texture.unwrap_or(1.0) as f32;
    let r = look.outline as f32 * 0.5 * s;
    let ink = |t: &Tone| match style {
        Style::Arcane => t.skin * Vec3::new(0.09, 0.045, 0.045),
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

    // A crease's ink: the crease lines of the data map (they stay on the skin), fading where
    // the surface turns away (a crease seen edge-on is a line already).
    let crease = |i: usize| -> f32 {
        let c = inp.cavity.get(i).copied().unwrap_or(0.0);
        let facing = smoothstep(0.15, 0.45, inp.normal[i].z);
        smoothstep(0.15, 0.6, c) * facing * (1.0 - inp.nail[i])
    };
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
                    cover(sil_d[i], rr).max(cover(int_d[i], rr * 0.55)).max(crease(i) * crease_ink)
                }
                Style::Arcane => {
                    let q = meters(x, y);
                    let wobble = img::noise2(q.x * 140.0, q.y * 140.0 + 7.0) * 0.7 + 0.5;
                    cover(sil_d[i], r * wobble).max(cover(int_d[i], r * 0.8 * wobble) * 0.85).max(crease(i) * crease_ink)
                }
                _ => cover(sil_d[i], r).max(cover(int_d[i], r * 0.6)).max(crease(i) * crease_ink),
            }
        })
        .collect();
    let lines = despeckle(lines, w, h, (14.0 * s * s).max(4.0) as usize);

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
            let tone = tone_at(i);
            let Tone { ps, pn, palm_ratio, skin } = tone;
            let mut out = [0f32; 4];
            if a > 0.0 {
                let n = inp.normal[i];
                let mut nl = n.dot(l);
                let m = inp.nail[i];
                let col = match style {
                    Style::Arcane => {
                        // Each brush stroke on the skin moves the band edges its own way, so
                        // they break along strokes.
                        let stroke = inp.detail[i] - 0.5;
                        nl += stroke * 0.14 * tex;
                        let soft = 0.17;
                        let t0 = smoothstep(-0.5 - soft, -0.5 + soft, nl);
                        let t1 = smoothstep(-0.1 - soft, -0.1 + soft, nl);
                        let t2 = smoothstep(0.42 - soft, 0.42 + soft, nl);
                        let band = |p: &Palette| p.deep.lerp(p.shadow, t0).lerp(p.mid, t1).lerp(p.light, t2);
                        let mut c = band(ps).lerp(band(pn), m);
                        // The stroke's colour: warmer and lighter, or cooler and darker.
                        c *= (Vec3::ONE + Vec3::new(0.16, 0.06, -0.1) * stroke * tex).max(Vec3::splat(0.2));
                        // Warm bounce on the shadow side's rim.
                        let rim = smoothstep(0.45, 0.95, 1.0 - n.z.max(0.0)) * (1.0 - t1);
                        c += Vec3::new(0.22, 0.09, 0.03) * rim * luminance(*skin) * 1.4;
                        c
                    }
                    _ => {
                        let aa = 0.025;
                        let t1 = smoothstep(0.18 - aa, 0.18 + aa, nl);
                        let t2 = smoothstep(0.72 - aa, 0.72 + aa, nl);
                        let band = |p: &Palette| p.shadow.lerp(p.mid, t1).lerp(p.light, t2);
                        let palm = inp.detail.get(i).copied().unwrap_or(0.0);
                        let mut c = (band(ps) * Vec3::ONE.lerp(*palm_ratio, palm)).min(Vec3::ONE).lerp(band(pn), m);
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
            let ink = ink(tone);
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

/// Removes ink specks: connected pieces of ink smaller than `min_px` pixels (a crease line
/// breaking into dots where it fades). Lines and outlines are far larger and stay.
fn despeckle(mut lines: Vec<f32>, w: usize, h: usize, min_px: usize) -> Vec<f32> {
    let mut seen = vec![false; w * h];
    let mut piece = Vec::new();
    for start in 0..w * h {
        if seen[start] || lines[start] <= 0.02 {
            continue;
        }
        piece.clear();
        piece.push(start);
        seen[start] = true;
        let mut k = 0;
        while k < piece.len() {
            let i = piece[k];
            k += 1;
            let (x, y) = (i % w, i / w);
            for (dx, dy) in [(-1isize, 0isize), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                let (u, v) = (x as isize + dx, y as isize + dy);
                if u < 0 || v < 0 || u >= w as isize || v >= h as isize {
                    continue;
                }
                let j = v as usize * w + u as usize;
                if !seen[j] && lines[j] > 0.02 {
                    seen[j] = true;
                    piece.push(j);
                }
            }
        }
        if piece.len() < min_px {
            for &i in &piece {
                lines[i] = 0.0;
            }
        }
    }
    lines
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
    let mix = inp.look.subsurface.unwrap_or(0.35) as f32;
    // Scattering distances of skin per channel (meters), red the farthest.
    let reach = [0.0024f32, 0.001, 0.0006];
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
    let warm = Vec3::new(1.15, 0.95, 0.88);
    finish(
        inp,
        |i| {
            let c = inp.color[i];
            let b = Vec3::new(blurred[0][i], blurred[1][i], blurred[2][i]);
            let mut c = c.lerp(b, mix);
            let dark = 1.0 - smoothstep(0.0, 0.2, luminance(c));
            c *= Vec3::ONE.lerp(warm, dark * 0.45);
            c
        },
        Vec3::splat(0.01),
    )
}

/// Matte clay: the lit render (its lumps and fingerprints are in the normal map).
pub(crate) fn clay(inp: &Inputs) -> Layer {
    finish(inp, |i| inp.color[i], Vec3::splat(0.01))
}

#[cfg(test)]
mod tests {
    use super::despeckle;

    #[test]
    fn despeckle_drops_dots_and_keeps_lines() {
        let (w, h) = (40, 10);
        let mut lines = vec![0.0f32; w * h];
        // A line 30 px long, faint at its end, and two 2-pixel specks.
        for x in 2..32 {
            lines[5 * w + x] = if x < 26 { 0.9 } else { 0.1 };
        }
        for i in [w + 36, w + 37, 8 * w + 1, 8 * w + 2] {
            lines[i] = 0.8;
        }
        let out = despeckle(lines.clone(), w, h, 14);
        assert!((2..32).all(|x| out[5 * w + x] == lines[5 * w + x]));
        assert!([w + 36, w + 37, 8 * w + 1, 8 * w + 2].iter().all(|i| out[*i] == 0.0));
    }
}
