//! Renders one pose in one look as a still, for close-ups and review sheets:
//!
//! `cargo run --release -p handkit-fframes --example still -- pose.json out.png [--look toon,skin=tan]
//!  [--size 1080] [--cam px,py,pz,tx,ty,tz,fov] [--assets DIR] [--bg "#76787b"] [--plain]`
//!
//! `--assets` loads `hand_R.glb` / `hand_L.glb` from DIR (default: generate them). `--plain`
//! draws the .glb's own material under a plain studio light instead of a look.

use std::time::Instant;

use handkit_fframes::HandModel;
use handkit_fframes::look::{Look, Styler};
use handkit_fframes::three_d::*;

fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(String::as_str)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (pose, out) = (&args[1], &args[2]);
    let size: u32 = flag(&args, "--size").and_then(|s| s.parse().ok()).unwrap_or(1080);
    let text = std::fs::read_to_string(pose).unwrap();
    let solved = handkit_core::solve(&text).unwrap();
    let models: Vec<HandModel> = solved
        .hands
        .iter()
        .map(|h| match flag(&args, "--assets") {
            Some(dir) => {
                let name = if h.side == handkit_core::pose::Side::Left { "hand_L.glb" } else { "hand_R.glb" };
                HandModel::from_glb(&std::fs::read(format!("{dir}/{name}")).unwrap()).unwrap()
            }
            None => HandModel::generate(h.side),
        })
        .collect();
    let cam: Vec<f32> = match flag(&args, "--cam") {
        Some(c) => c.split(',').map(|x| x.trim().parse().unwrap()).collect(),
        None => {
            let c = solved.pose.camera.clone().unwrap_or(serde_json::json!({"position": [0.0, 0.1, 0.45], "target": [0, 0.09, 0], "fov": 35}));
            let v = |k: &str, i: usize| c[k][i].as_f64().unwrap() as f32;
            vec![v("position", 0), v("position", 1), v("position", 2), v("target", 0), v("target", 1), v("target", 2), c["fov"].as_f64().unwrap_or(35.0) as f32]
        }
    };
    let camera = Camera::perspective(cam[6]).at(cam[0], cam[1], cam[2]).look_at(vec3(cam[3], cam[4], cam[5]));
    let bg = flag(&args, "--bg").map(|s| handkit_core::look::parse_color(s).unwrap());

    let t0 = Instant::now();
    let px = if args.iter().any(|a| a == "--plain") {
        let mut scene = Scene::new().environment(Environment::studio());
        scene.add(Node::light(Light::hemisphere(Rgb::WHITE, Rgb::hex(0x404050), 1.6)));
        scene.add(Node::light(Light::directional(Rgb::WHITE, 2.2, vec3(0.6, 1.0, 0.8))));
        // `--drop normal,ao,color,rough` leaves maps out, to see what each one does.
        let drop = flag(&args, "--drop").unwrap_or("");
        for m in &models {
            let hand = m.add_to(&mut scene);
            hand.pose(&mut scene, &solved);
            if let Some(mat) = hand.mesh().and_then(|id| scene[id].material_mut()) {
                if drop.contains("normal") {
                    mat.normal_map = None;
                }
                if drop.contains("ao") {
                    mat.ao_map = None;
                }
                if drop.contains("color") {
                    mat.map = None;
                }
                if drop.contains("rough") {
                    mat.metal_rough_map = None;
                    mat.roughness = 0.55;
                }
            }
        }
        Renderer::new(size, size).cpu().tone_mapping(ToneMapping::Neutral).render_pixels(&scene, &camera)
    } else {
        let base = solved.pose.look.clone();
        let over = flag(&args, "--look").map(|s| Look::parse_flag(s).unwrap());
        let value = Look::merge(base.as_ref(), over.as_ref()).unwrap_or(serde_json::json!("realistic"));
        let look = Look::from_value(&value).unwrap();
        let mut styler = Styler::new(look, size, size);
        let hands: Vec<HandModel> = models.iter().map(|m| styler.dress(m)).collect();
        eprintln!("dressed in {:.0} ms", t0.elapsed().as_secs_f64() * 1000.0);
        let t1 = Instant::now();
        let px = styler.render(|_| {
            let mut scene = styler.stage();
            for m in &hands {
                m.add_to(&mut scene).pose(&mut scene, &solved);
            }
            (scene, camera.clone())
        });
        eprintln!("rendered in {:.0} ms", t1.elapsed().as_secs_f64() * 1000.0);
        px
    };
    let mut rgba = px.to_straight_rgba();
    if let Some(bg) = bg {
        for p in rgba.chunks_mut(4) {
            let a = p[3] as f32 / 255.0;
            for k in 0..3 {
                p[k] = (p[k] as f32 * a + bg[k] as f32 * (1.0 - a)).round() as u8;
            }
            p[3] = 255;
        }
    }
    image::save_buffer(out, &rgba, size, size, image::ColorType::Rgba8).unwrap();
    println!("wrote {out}");
}
