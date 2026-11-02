//! Looks: the nail mask comes through the .glb, every style draws the hand, stylized styles
//! draw ink around it, and motion blur averages sub-frames.

use handkit_core::pose::Side;
use handkit_fframes::HandModel;
use handkit_fframes::look::{Look, Style, Styler};
use handkit_fframes::three_d::*;

fn coarse_hand() -> HandModel {
    let mesh = handkit_mesh::generate(Side::Right, &handkit_mesh::Options { voxel: 0.004, textures: 0 });
    HandModel::from_glb(&handkit_mesh::glb::to_glb(&mesh)).unwrap()
}

fn spread() -> handkit_core::Solved {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../handkit-core/gestures/spread_palm.json")).unwrap();
    handkit_core::solve(&text).unwrap()
}

fn render(styler: &Styler, hand: &HandModel, solved: &handkit_core::Solved, shift: f32) -> Pixels {
    styler.render(|dt| {
        let mut scene = styler.stage();
        let h = hand.add_to(&mut scene);
        h.pose(&mut scene, solved);
        scene[h.instance.root].position.x += dt * shift;
        let camera = Camera::perspective(36.0).at(0.0, 0.1, 0.45).look_at(vec3(0.0, 0.09, 0.0));
        (scene, camera)
    })
}

#[test]
fn reads_the_nail_mask() {
    let hand = coarse_hand();
    let id = hand.mesh().unwrap();
    assert_eq!(hand.nail.len(), hand.model.scene[id].as_mesh().unwrap().geometry.positions.len());
    let on = hand.nail.iter().filter(|x| **x > 0.5).count();
    assert!(on > 20 && on < hand.nail.len() / 5, "{on} nail vertices of {}", hand.nail.len());
}

#[test]
fn every_style_draws_the_hand() {
    let hand = coarse_hand();
    let solved = spread();
    let (w, h) = (160u32, 160u32);
    for style in Style::ALL {
        // Lines are sized for 1080p: keep them a few pixels wide in this small frame.
        let mut look = Look::of(style);
        look.outline *= 1080.0 / h as f64;
        let mut styler = Styler::new(look, w, h);
        let dressed = styler.dress(&hand);
        let px = render(&styler, &dressed, &solved, 0.0);
        let rgba = px.to_straight_rgba();
        let covered = rgba.chunks(4).filter(|p| p[3] > 200).count();
        assert!(covered > 1500, "{}: {covered} pixels", style.name());
        let ink = rgba.chunks(4).filter(|p| p[3] > 200 && p[0] < 40 && p[1] < 40 && p[2] < 40).count();
        match style {
            Style::Toon | Style::Spiderverse => assert!(ink > 100, "{}: {ink} ink pixels", style.name()),
            Style::Realistic | Style::Clay => assert!(ink < 20, "{}: {ink} ink pixels", style.name()),
            Style::Arcane => {}
        }
    }
}

#[test]
fn motion_blur_spreads_a_moving_hand() {
    let hand = coarse_hand();
    let solved = spread();
    let soft = |px: &Pixels| px.to_straight_rgba().chunks(4).filter(|p| p[3] > 10 && p[3] < 245).count();
    let mut look = Look::of(Style::Toon);
    let mut still = Styler::new(look.clone(), 160, 160);
    let still_hand = still.dress(&hand);
    look.motion_blur = 4;
    let mut blurred = Styler::new(look, 160, 160);
    let blurred_hand = blurred.dress(&hand);
    // 3 cm per frame to the right.
    let a = soft(&render(&still, &still_hand, &solved, 0.03));
    let b = soft(&render(&blurred, &blurred_hand, &solved, 0.03));
    assert!(b > a * 2, "partly covered pixels: {a} still, {b} blurred");
}

#[test]
fn every_style_shows_each_persons_skin() {
    let hand = coarse_hand();
    let solved = spread();
    let (w, h) = (240u32, 160u32);
    let (dark, light) = ([0x5e, 0x3e, 0x30], [0xf0, 0xc8, 0xb2]);
    for style in Style::ALL {
        let mut look = Look::of(style);
        look.outline *= 1080.0 / h as f64;
        let mut styler = Styler::new(look, w, h);
        // Two people's hands side by side: dark on the left, light on the right.
        let left = styler.dress_in(&hand, dark, None);
        let right = styler.dress_in(&hand, light, None);
        let px = styler.render(|_| {
            let mut scene = styler.stage();
            for (model, x) in [(&left, -0.075), (&right, 0.075)] {
                let hi = model.add_to(&mut scene);
                hi.pose(&mut scene, &solved);
                scene[hi.instance.root].position.x += x;
            }
            (scene, Camera::perspective(40.0).at(0.0, 0.1, 0.5).look_at(vec3(0.0, 0.09, 0.0)))
        });
        let rgba = px.to_straight_rgba();
        let mean = |x0: u32, x1: u32| {
            let (mut sum, mut n) = (0.0, 0);
            for y in 0..h {
                for x in x0..x1 {
                    let p = &rgba[((y * w + x) * 4) as usize..][..4];
                    if p[3] > 200 && p.iter().take(3).any(|c| *c > 40) {
                        sum += f64::from(p[0]) + f64::from(p[1]) + f64::from(p[2]);
                        n += 1;
                    }
                }
            }
            assert!(n > 300, "{}: {n} pixels", style.name());
            sum / n as f64
        };
        let (l, r) = (mean(0, w / 2), mean(w / 2, w));
        assert!(r > l + 120.0, "{}: left (dark) {l:.0}, right (light) {r:.0}", style.name());
    }
}

/// A straight right arm from the elbow at the origin up +Y, palm to the camera, the whole
/// forearm in view, in `look` (JSON): the straight RGBA, and the index of the pixel on the
/// forearm's axis `y` meters from the elbow.
fn render_arm(look: serde_json::Value) -> (Vec<u8>, impl Fn(f32) -> usize) {
    let (w, h) = (120u32, 240u32);
    let hand = coarse_hand();
    let solved = handkit_core::solve(r#"{"version": 1, "hands": {"right": {"place": {"elbow": [0, 0, 0]}}}}"#).unwrap();
    let mut look = Look::from_value(&look).unwrap();
    look.outline *= 1080.0 / h as f64;
    let mut styler = Styler::new(look, w, h);
    let dressed = styler.dress(&hand);
    let camera = Camera::perspective(35.0).at(0.0, 0.16, 0.75).look_at(vec3(0.0, 0.16, 0.0));
    let px = styler.render(|_| {
        let mut scene = styler.stage();
        dressed.add_to(&mut scene).pose(&mut scene, &solved);
        (scene, camera.clone())
    });
    let at = move |y: f32| {
        let p = camera.project(vec3(0.0, y, 0.0), w as f32, h as f32).unwrap();
        (p.y as usize * w as usize + p.x as usize) * 4
    };
    (px.to_straight_rgba(), at)
}

#[test]
fn a_fading_arm_fades_toward_the_elbow() {
    for style in Style::ALL.map(Style::name) {
        let (bare, at) = render_arm(serde_json::json!({ "style": style }));
        let (faded, _) = render_arm(serde_json::json!({ "style": style, "arm": "fade", "motion_blur": 2 }));
        let alpha = |px: &[u8], y: f32| px[at(y) + 3];
        // Bare: opaque to the elbow. Faded: gone at the elbow, partly there on the way, opaque
        // nearer the wrist; the ink round the end fades with it.
        assert!(alpha(&bare, 0.012) > 240 && alpha(&bare, 0.2) > 240, "{style}");
        assert!(alpha(&faded, 0.012) < 20, "{style}: {}", alpha(&faded, 0.012));
        assert!(alpha(&faded, 0.0) < 20, "{style}: {}", alpha(&faded, 0.0));
        assert!((50..220).contains(&alpha(&faded, 0.045)), "{style}: {}", alpha(&faded, 0.045));
        assert!(alpha(&faded, 0.2) > 240, "{style}: {}", alpha(&faded, 0.2));
    }
}

#[test]
fn a_sleeve_covers_the_forearm_from_the_elbow() {
    for style in Style::ALL {
        let (px, at) = render_arm(serde_json::json!({ "style": style.name(), "skin": "light", "arm": { "sleeve": "#2a4fb0", "length": 0.1 } }));
        let rgba = |y: f32| [px[at(y)], px[at(y) + 1], px[at(y) + 2], px[at(y) + 3]];
        // Blue cloth up the forearm from the elbow; past the cuff, skin.
        for y in [0.015, 0.04, 0.07] {
            let [r, _, b, a] = rgba(y);
            assert!(a > 240 && b > r + 30, "{}: at {y} m {:?}", style.name(), rgba(y));
        }
        let [r, _, b, a] = rgba(0.19);
        assert!(a > 240 && r > b + 30, "{}: skin {:?}", style.name(), rgba(0.19));
        // Past the elbow it fades out, cloth to the end (no skin shows through).
        let [r, _, b, a] = rgba(-0.006);
        assert!((20..230).contains(&a) && b > r, "{}: the end {:?}", style.name(), rgba(-0.006));
        assert!(rgba(-0.04)[3] < 10, "{}: {:?}", style.name(), rgba(-0.04));
    }
}
