//! Renders a pose file's hands with fframes' CPU renderer, for golden comparisons with the
//! three.js preview: `cargo run --release -p handkit-fframes --example render -- pose.json out.png [size]`
//! The camera is the pose's `camera` (or a default); the background is transparent.

use handkit_fframes::HandModel;
use handkit_fframes::three_d::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (pose, out) = (&args[1], &args[2]);
    let size: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(512);
    let text = std::fs::read_to_string(pose).unwrap();
    let solved = handkit_core::solve(&text).unwrap();

    let mut scene = Scene::new().environment(Environment::studio());
    scene.add(Node::light(Light::hemisphere(Rgb::WHITE, Rgb::hex(0x404050), 1.6)));
    scene.add(Node::light(Light::directional(Rgb::WHITE, 2.2, vec3(0.6, 1.0, 0.8))));
    for h in &solved.hands {
        let model = HandModel::generate(h.side);
        let hand = model.add_to(&mut scene);
        hand.pose(&mut scene, &solved);
    }
    let cam = solved.pose.camera.clone().unwrap_or(serde_json::json!({"position": [0.25, 0.15, 0.45], "target": [0, 0.09, 0], "fov": 35}));
    let v = |k: &str, i: usize| cam[k][i].as_f64().unwrap() as f32;
    let camera = Camera::perspective(cam["fov"].as_f64().unwrap_or(35.0) as f32)
        .at(v("position", 0), v("position", 1), v("position", 2))
        .look_at(vec3(v("target", 0), v("target", 1), v("target", 2)));
    let pixels = Renderer::new(size, size).cpu().tone_mapping(ToneMapping::Neutral).render_pixels(&scene, &camera);
    image::save_buffer(out, &pixels.to_straight_rgba(), size, size, image::ColorType::Rgba8).unwrap();
    println!("wrote {out}");
}
