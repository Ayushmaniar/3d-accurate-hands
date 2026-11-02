//! Renders one pose in every look: a grid of styles (rows) by skin tones (columns), and the
//! time per 1080p frame of each style.
//!
//! `cargo run --release -p handkit-fframes --example looks -- pose.json grid.png [cell] [--bench]
//!  [--only style] [--tone name] [--extra pose.json]`
//!
//! `--extra` adds a last column: a second pose (with its own camera) in each style, medium skin.
//!
//! Each cell renders at `cell` x `cell` pixels (default 1080, so lines and dots have their
//! 1080p size) and is halved into the grid.

use std::time::Instant;

use handkit_fframes::HandModel;
use handkit_fframes::look::{Look, Style, Styler, TONES};
use handkit_fframes::three_d::*;

/// The backdrop of each style's row (sRGB).
fn backdrop(style: Style) -> [u8; 3] {
    match style {
        Style::Realistic => [0x76, 0x78, 0x7b],
        Style::Toon => [0xf3, 0xee, 0xe2],
        Style::Spiderverse => [0xfb, 0xf8, 0xf0],
        Style::Arcane => [0x16, 0x1a, 0x24],
        Style::Clay => [0xbd, 0xb8, 0xb2],
    }
}

/// A pose's camera (a default one when it has none).
fn pose_camera(solved: &handkit_core::Solved) -> (Camera, f32) {
    let cam = solved.pose.camera.clone().unwrap_or(serde_json::json!({"position": [0.0, 0.1, 0.45], "target": [0, 0.09, 0], "fov": 35}));
    let v = |k: &str, i: usize| cam[k][i].as_f64().unwrap() as f32;
    let fov = cam["fov"].as_f64().unwrap_or(35.0) as f32;
    let camera = Camera::perspective(fov)
        .at(v("position", 0), v("position", 1), v("position", 2))
        .look_at(vec3(v("target", 0), v("target", 1), v("target", 2)));
    (camera, fov)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (pose, out) = (&args[1], &args[2]);
    let cell: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1080);
    let bench = args.iter().any(|a| a == "--bench");
    let flag = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1));
    let only = flag("--only").map(|s| Style::parse(s).unwrap());
    let solve = |path: &str| handkit_core::solve(&std::fs::read_to_string(path).unwrap()).unwrap();
    let solved = solve(pose);
    let extra = flag("--extra").map(|p| solve(p));
    let mut models: Vec<HandModel> = Vec::new();
    for h in solved.hands.iter().chain(extra.iter().flat_map(|e| &e.hands)) {
        if !models.iter().any(|m| m.side == h.side) {
            models.push(HandModel::generate(h.side));
        }
    }
    let (camera, fov) = pose_camera(&solved);
    let base = solved.pose.look.as_ref().map(|l| Look::from_value(l).unwrap());

    let styles: Vec<Style> = Style::ALL.into_iter().filter(|s| only.is_none_or(|o| o == *s)).collect();
    let tone = flag("--tone").cloned();
    let tones: Vec<(&str, [u8; 3])> = TONES.iter().copied().filter(|(n, _)| tone.as_deref().is_none_or(|t| t == *n)).collect();
    // The columns: each tone in the main pose, then the extra pose in medium skin.
    let medium = TONES.iter().find(|(n, _)| *n == "medium").unwrap().1;
    let extra = extra.map(|e| {
        let cam = pose_camera(&e).0;
        (e, cam)
    });
    let mut columns: Vec<(&str, [u8; 3], &handkit_core::Solved, &Camera)> = tones.iter().map(|(n, t)| (*n, *t, &solved, &camera)).collect();
    if let Some((e, cam)) = &extra {
        columns.push(("medium (extra pose)", medium, e, cam));
    }
    // One style in one tone: the cell at full size.
    let full = styles.len() == 1 && columns.len() == 1;
    let half = if full { cell } else { cell / 2 };
    let step = if full { 1 } else { 2 };
    let (gw, gh) = (half as usize * columns.len(), half as usize * styles.len());
    let mut grid = vec![0u8; gw * gh * 3];
    for (row, style) in styles.iter().enumerate() {
        for (col, (tone_name, tone, solved, camera)) in columns.iter().enumerate() {
            let mut look = base.clone().filter(|l| l.style == *style).unwrap_or(Look::of(*style));
            look.skin = *tone;
            let mut styler = Styler::new(look, cell, cell);
            let hands: Vec<HandModel> =
                solved.hands.iter().map(|h| styler.dress(models.iter().find(|m| m.side == h.side).unwrap())).collect();
            let t0 = Instant::now();
            let px = styler.render(|_| {
                let mut scene = styler.stage();
                for m in &hands {
                    m.add_to(&mut scene).pose(&mut scene, solved);
                }
                (scene, (*camera).clone())
            });
            eprintln!("{} {}: {:.0} ms at {cell}px", style.name(), tone_name, t0.elapsed().as_secs_f64() * 1000.0);
            let rgba = px.to_straight_rgba();
            let bg = backdrop(*style);
            for y in 0..half as usize {
                for x in 0..half as usize {
                    let mut acc = [0f32; 3];
                    let taps: &[(usize, usize)] = if full { &[(0, 0)] } else { &[(0, 0), (1, 0), (0, 1), (1, 1)] };
                    for (dx, dy) in taps {
                        let i = ((y * step + dy) * cell as usize + x * step + dx) * 4;
                        let a = rgba[i + 3] as f32 / 255.0;
                        for k in 0..3 {
                            acc[k] += rgba[i + k] as f32 * a + bg[k] as f32 * (1.0 - a);
                        }
                    }
                    let o = ((row * half as usize + y) * gw + col * half as usize + x) * 3;
                    for k in 0..3 {
                        grid[o + k] = (acc[k] / taps.len() as f32).round() as u8;
                    }
                }
            }
        }
    }
    image::save_buffer(out, &grid, gw as u32, gh as u32, image::ColorType::Rgb8).unwrap();
    println!("wrote {out}");

    if bench {
        let mut scene_cam = camera.clone();
        scene_cam.fov = fov;
        for style in &styles {
            for blur in [1u32, 4] {
                let mut look = Look::of(*style);
                look.motion_blur = blur;
                let mut styler = Styler::new(look, 1920, 1080);
                let hands: Vec<HandModel> =
                    solved.hands.iter().map(|h| styler.dress(models.iter().find(|m| m.side == h.side).unwrap())).collect();
                let frame = || {
                    styler.render(|_| {
                        let mut scene = styler.stage();
                        for m in &hands {
                            m.add_to(&mut scene).pose(&mut scene, &solved);
                        }
                        (scene, scene_cam.clone())
                    })
                };
                frame();
                let n = 3;
                let t0 = Instant::now();
                for _ in 0..n {
                    frame();
                }
                println!("bench {:<12} motion_blur {blur}: {:.0} ms per 1920x1080 frame", style.name(), t0.elapsed().as_secs_f64() * 1000.0 / n as f64);
            }
        }
    }
}
