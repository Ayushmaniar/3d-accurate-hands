//! UVs for the hand: charts per bone, packed in one square atlas.
//!
//! Each triangle goes to its bone (the one with the most skin weight on its corners) and to the
//! nearest of 26 directions in that bone's frame (the faces, edges and corners of a cube), so
//! a triangle leans at most about 35 degrees from its chart's projection and the planar
//! projections stay close to the true surface (typically under 15% squashed; the few triangles
//! that smoothing or a merged tiny chart moves lean up to 60-70 degrees). Connected triangles
//! with the same bone and direction make a chart. Charts are packed on a skyline with a gutter,
//! and every vertex on a chart border is split, one copy per chart.
//!
//! The textures are solid (each texel is computed from its bind-pose 3D position), so both
//! sides of a seam show the same thing and the seams do not show.

use glam::{DAffine3, DVec2, DVec3};
use std::collections::HashMap;

pub struct Unwrap {
    /// For each new vertex, the original vertex it copies.
    pub source: Vec<u32>,
    pub uvs: Vec<[f32; 2]>,
    /// The triangles over the new vertices, in the original order.
    pub indices: Vec<u32>,
    /// Each triangle's chart.
    pub tri_chart: Vec<u32>,
    pub charts: usize,
    /// Texels per meter at density 1.
    pub texels_per_meter: f64,
}

/// The 26 directions from the center of a cube to its faces, edges and corners.
fn directions() -> Vec<DVec3> {
    let mut out = Vec::new();
    for x in -1..=1 {
        for y in -1..=1 {
            for z in -1..=1 {
                if (x, y, z) != (0, 0, 0) {
                    out.push(DVec3::new(x as f64, y as f64, z as f64).normalize());
                }
            }
        }
    }
    out
}

/// `bone_of[t]` is triangle t's bone; `frames[b]` bone b's frame (local to bind space);
/// `density[b]` how many texels per meter bone b gets relative to the others. `size` is the
/// atlas's side in texels, `gutter` the free texels around each chart.
pub fn unwrap(positions: &[DVec3], indices: &[u32], bone_of: &[usize], frames: &[DAffine3], density: &[f64], size: u32, gutter: u32) -> Unwrap {
    let tris = indices.len() / 3;
    let dirs = directions();
    let corner = |t: usize, k: usize| positions[indices[3 * t + k] as usize];
    let face_normal = |t: usize| (corner(t, 1) - corner(t, 0)).cross(corner(t, 2) - corner(t, 0)).normalize_or_zero();
    let local_dir = |t: usize, b: usize| frames[b].matrix3.inverse().mul_vec3(face_normal(t)).normalize_or_zero();
    let best_dir = |n: DVec3| (0..dirs.len()).max_by(|&a, &b| n.dot(dirs[a]).total_cmp(&n.dot(dirs[b]))).unwrap();
    let mut label: Vec<(usize, usize)> = (0..tris).map(|t| (bone_of[t], best_dir(local_dir(t, bone_of[t])))).collect();

    // Edge neighbours.
    let mut edges: HashMap<(u32, u32), Vec<u32>> = HashMap::with_capacity(tris * 2);
    for t in 0..tris {
        for k in 0..3 {
            let (a, b) = (indices[3 * t + k], indices[3 * t + (k + 1) % 3]);
            edges.entry((a.min(b), a.max(b))).or_default().push(t as u32);
        }
    }
    let mut neighbours: Vec<Vec<u32>> = vec![Vec::new(); tris];
    for ts in edges.values() {
        for &a in ts {
            for &b in ts {
                if a != b {
                    neighbours[a as usize].push(b);
                }
            }
        }
    }

    // Smooth the labels: a triangle takes a label most of its neighbours share, if its own
    // normal is not too far from that label's direction.
    let fits = |t: usize, (b, d): (usize, usize), min: f64| local_dir(t, b).dot(dirs[d]) > min;
    for _ in 0..4 {
        let next: Vec<(usize, usize)> = (0..tris)
            .map(|t| {
                let ns = &neighbours[t];
                for &n in ns {
                    let l = label[n as usize];
                    let count = ns.iter().filter(|&&m| label[m as usize] == l).count();
                    if l != label[t] && count >= 2 && fits(t, l, 0.5) {
                        return l;
                    }
                }
                label[t]
            })
            .collect();
        label = next;
    }

    // Charts: connected triangles with the same label. Small ones join the neighbour they share
    // the most edges with, when it fits.
    let components = |label: &[(usize, usize)]| -> (Vec<u32>, usize) {
        let mut comp = vec![u32::MAX; tris];
        let mut count = 0;
        let mut stack = Vec::new();
        for s in 0..tris {
            if comp[s] != u32::MAX {
                continue;
            }
            comp[s] = count;
            stack.push(s);
            while let Some(t) = stack.pop() {
                for &n in &neighbours[t] {
                    let n = n as usize;
                    if comp[n] == u32::MAX && label[n] == label[t] {
                        comp[n] = count;
                        stack.push(n);
                    }
                }
            }
            count += 1;
        }
        (comp, count as usize)
    };
    for _ in 0..3 {
        let (comp, count) = components(&label);
        let mut members: Vec<Vec<usize>> = vec![Vec::new(); count];
        for t in 0..tris {
            members[comp[t] as usize].push(t);
        }
        let mut changed = false;
        for m in members.iter().filter(|m| m.len() < 24) {
            let mut votes: HashMap<(usize, usize), usize> = HashMap::new();
            for &t in m {
                for &n in &neighbours[t] {
                    if comp[n as usize] != comp[t] {
                        *votes.entry(label[n as usize]).or_default() += 1;
                    }
                }
            }
            let mut options: Vec<_> = votes.into_iter().collect();
            options.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            if let Some((l, _)) = options.into_iter().find(|(l, _)| m.iter().all(|&t| fits(t, *l, 0.3))) {
                for &t in m {
                    label[t] = l;
                }
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let (tri_chart, charts) = components(&label);

    // Project each chart along its direction, turned to its smallest bounding box.
    let mut chart_tris: Vec<Vec<usize>> = vec![Vec::new(); charts];
    for t in 0..tris {
        chart_tris[tri_chart[t] as usize].push(t);
    }
    struct Layout {
        a: DVec3,
        c: DVec3,
        lo: DVec2,
        ext: DVec2,
        density: f64,
        swap: bool,
        at: DVec2,
    }
    let mut layouts: Vec<Layout> = chart_tris
        .iter()
        .map(|ts| {
            let (b, d) = label[ts[0]];
            let axis = frames[b].matrix3.mul_vec3(dirs[d]).normalize();
            // A basis across the axis, starting from the bone's length (y).
            let y = frames[b].matrix3.y_axis.normalize();
            let mut a = (y - axis * y.dot(axis)).normalize_or_zero();
            if a == DVec3::ZERO {
                a = axis.any_orthonormal_vector();
            }
            let c = axis.cross(a);
            let mut pts: Vec<DVec2> = Vec::with_capacity(ts.len() * 3);
            for &t in ts {
                for k in 0..3 {
                    let p = corner(t, k);
                    pts.push(DVec2::new(p.dot(a), p.dot(c)));
                }
            }
            let mut best = (f64::INFINITY, 0.0);
            for i in 0..18 {
                let ang = i as f64 * std::f64::consts::FRAC_PI_2 / 18.0;
                let (s, co) = ang.sin_cos();
                let (mut lo, mut hi) = (DVec2::splat(f64::INFINITY), DVec2::splat(f64::NEG_INFINITY));
                for p in &pts {
                    let q = DVec2::new(co * p.x - s * p.y, s * p.x + co * p.y);
                    lo = lo.min(q);
                    hi = hi.max(q);
                }
                let area = (hi - lo).x * (hi - lo).y;
                if area < best.0 {
                    best = (area, ang);
                }
            }
            let (s, co) = best.1.sin_cos();
            let (a, c) = (a * co - c * s, a * s + c * co);
            let (mut lo, mut hi) = (DVec2::splat(f64::INFINITY), DVec2::splat(f64::NEG_INFINITY));
            for &t in ts {
                for k in 0..3 {
                    let p = corner(t, k);
                    let q = DVec2::new(p.dot(a), p.dot(c));
                    lo = lo.min(q);
                    hi = hi.max(q);
                }
            }
            let ext = hi - lo;
            Layout { a, c, lo, ext, density: density[b], swap: ext.y > ext.x, at: DVec2::ZERO }
        })
        .collect();

    // Skyline packing, longest side first, each chart turned the way that keeps the skyline
    // lowest; the largest scale that fits, by bisection.
    let mut order: Vec<usize> = (0..charts).collect();
    order.sort_by(|&i, &j| layouts[j].ext.max_element().total_cmp(&layouts[i].ext.max_element()).then(i.cmp(&j)));
    let side = size as usize;
    let pack = |scale: f64, layouts: &mut [Layout]| -> bool {
        // (x, width, height) runs of the skyline, left to right.
        let mut sky: Vec<(usize, usize, usize)> = vec![(0, side, 0)];
        for &i in &order {
            let e = layouts[i].ext * layouts[i].density * scale;
            let size_of = |a: f64| a.ceil() as usize + 1 + 2 * gutter as usize;
            let mut best: Option<(usize, usize, usize, bool, usize)> = None; // (top, x, run, swap, y)
            for swap in [false, true] {
                let (w, h) = if swap { (size_of(e.y), size_of(e.x)) } else { (size_of(e.x), size_of(e.y)) };
                for r in 0..sky.len() {
                    let x = sky[r].0;
                    if x + w > side {
                        break;
                    }
                    let (mut y, mut k) = (0, r);
                    while k < sky.len() && sky[k].0 < x + w {
                        y = y.max(sky[k].2);
                        k += 1;
                    }
                    if y + h <= side && best.is_none_or(|b| (y + h, x) < (b.0, b.1)) {
                        best = Some((y + h, x, r, swap, y));
                    }
                }
            }
            let Some((top, x, _, swap, y)) = best else { return false };
            let w = if swap { size_of(e.y) } else { size_of(e.x) };
            layouts[i].swap = swap;
            layouts[i].at = DVec2::new(x as f64, y as f64);
            // Raise the skyline over [x, x + w).
            let mut next = Vec::with_capacity(sky.len() + 2);
            for &(sx, sw, sh) in &sky {
                let end = sx + sw;
                if end <= x || sx >= x + w {
                    next.push((sx, sw, sh));
                    continue;
                }
                if sx < x {
                    next.push((sx, x - sx, sh));
                }
                if end > x + w {
                    next.push((x + w, end - x - w, sh));
                }
            }
            next.push((x, w, top));
            next.sort_by_key(|s| s.0);
            sky.clear();
            for s in next {
                match sky.last_mut() {
                    Some(last) if last.2 == s.2 && last.0 + last.1 == s.0 => last.1 += s.1,
                    _ => sky.push(s),
                }
            }
        }
        true
    };
    let (mut lo, mut hi) = (1.0, 1.0e5);
    for _ in 0..30 {
        let mid = (lo * hi as f64).sqrt();
        if pack(mid, &mut layouts) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let scale = lo;
    assert!(pack(scale, &mut layouts), "the charts fit");
    let side = size as f64;

    // Split the vertices along chart borders and place each copy in its chart.
    let mut copies: HashMap<(u32, u32), u32> = HashMap::with_capacity(positions.len() * 5 / 4);
    let mut source = Vec::with_capacity(positions.len() * 5 / 4);
    let mut uvs = Vec::with_capacity(positions.len() * 5 / 4);
    let mut out = Vec::with_capacity(indices.len());
    for t in 0..tris {
        let ch = tri_chart[t];
        let l = &layouts[ch as usize];
        for k in 0..3 {
            let v = indices[3 * t + k];
            let id = *copies.entry((v, ch)).or_insert_with(|| {
                let p = positions[v as usize];
                let q = (DVec2::new(p.dot(l.a), p.dot(l.c)) - l.lo) * l.density * scale;
                let q = if l.swap { DVec2::new(q.y, q.x) } else { q };
                let uv = (l.at + DVec2::splat(gutter as f64 + 0.5) + q) / side;
                source.push(v);
                uvs.push([uv.x as f32, uv.y as f32]);
                (source.len() - 1) as u32
            });
            out.push(id);
        }
    }
    Unwrap { source, uvs, indices: out, tri_chart, charts, texels_per_meter: scale }
}
