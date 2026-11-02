//! Looks: how the hands are drawn (`"look"` in a pose or clip, see `handkit_core::look`).
//!
//! A [`Styler`] dresses the hand models for a look (skin and nail colours from the mesh's
//! per-vertex nail mask, lumps for clay), lights the scene relative to the camera, renders the
//! passes a look needs on the CPU and post-processes them:
//!
//! - realistic and clay: one lit colour pass, then screen-space subsurface scattering and warm
//!   shadows (realistic) or fingerprints and grain (clay);
//! - toon, spiderverse, arcane: a normals pass (`Material::normals`) and a data pass (nail
//!   mask, object-space noise and depth through fog), shaded in screen space: cel bands from
//!   N·L, ink lines from alpha, normal and depth edges, halftone, brush strokes.
//!
//! ```rust,ignore
//! let mut styler = Styler::new(Look::from_value(&look_json)?, 1920, 1080);
//! let hands: Vec<HandModel> = models.iter().map(|m| styler.dress(m)).collect();
//! // per frame: the scene and camera `dt` frames from the frame's time (motion blur)
//! let image = styler.render(|dt| build_scene_at(t + dt / fps)).into_image();
//! ```

mod img;
mod shade;

pub use handkit_core::look::{Look, Style, TONES};

use std::sync::{Arc, OnceLock};

use fframes_three_d::{
    Background, Camera, Environment, Fog, Geometry, Light, Material, Node, NodeId, Pixels, Renderer, Rgb, Scene, Shading, Shadow, ToneMapping,
    Vec2, Vec3, Vec4,
};

use crate::HandModel;
use rayon::prelude::*;
use img::{Layer, Rect};

/// Renders scenes in a look. Build it once; `render` is `&self` and thread safe.
pub struct Styler {
    pub look: Look,
    pub width: u32,
    pub height: u32,
    color: Renderer,
    flat: Renderer,
    /// The dressed hands: (colour geometry, data geometry).
    dressed: Vec<(Arc<Geometry>, Arc<Geometry>)>,
}

/// Fog runs from near to this fraction of the data pass's depth range, so the colour channels
/// it fades stay readable.
const FOG_SPAN: f32 = 0.6;

fn brush() -> Option<&'static img::Tile> {
    static TILE: OnceLock<Option<img::Tile>> = OnceLock::new();
    TILE.get_or_init(|| img::Tile::decode(include_bytes!("../../assets/brush_tile.jpg"), 512)).as_ref()
}

impl Styler {
    pub fn new(look: Look, width: u32, height: u32) -> Self {
        let color = Renderer::new(width, height).cpu().tone_mapping(ToneMapping::Neutral).exposure(match look.style {
            Style::Clay => 1.05,
            _ => 1.0,
        });
        let flat = Renderer::new(width, height).cpu().tone_mapping(ToneMapping::Linear);
        Styler { look, width, height, color, flat, dressed: Vec::new() }
    }

    fn stylized(&self) -> bool {
        matches!(self.look.style, Style::Toon | Style::Spiderverse | Style::Arcane)
    }

    /// A copy of `model` dressed for the look: skin and nail colours per vertex, the look's
    /// material, lumpy normals for clay. Add the dressed model to the scenes you render.
    pub fn dress(&mut self, model: &HandModel) -> HandModel {
        let colors = (self.look.skin, self.look.nail_color());
        self.dress_in(model, colors)
    }

    /// `dress` with another skin and nail colour (sRGB), e.g. one person's in a scene of many.
    pub fn dress_in(&mut self, model: &HandModel, (skin, nail): ([u8; 3], [u8; 3])) -> HandModel {
        let mut out = model.clone();
        let Some(id) = out.mesh() else { return out };
        let mesh = out.model.scene[id].mesh_mut().unwrap();
        let skin = img::srgb(skin);
        let nail = img::srgb(nail);
        let nail_weight = if self.look.style == Style::Clay { 0.3 } else { 1.0 };
        let mask = |i: usize| model.nail.get(i).copied().unwrap_or(0.0);
        let mut g = (*mesh.geometry).clone();
        // Clay is the skin's colour as a matte, slightly more saturated body.
        // Lit skin reads more saturated than its albedo: realistic takes a little out, toward pink.
        let body = match self.look.style {
            Style::Clay => skin * Vec3::new(1.04, 0.93, 0.8),
            Style::Realistic => {
                // Darker tones take more out: lit, they turn orange.
                let lum = skin.dot(Vec3::new(0.2126, 0.7152, 0.0722));
                skin.lerp(Vec3::splat(lum), (0.18 + 0.25 * (1.0 - lum / 0.6)).clamp(0.18, 0.43)) * Vec3::new(1.02, 0.97, 1.0)
            }
            _ => skin,
        };
        g.colors = (0..g.positions.len()).map(|i| body.lerp(nail, mask(i) * nail_weight).extend(1.0)).collect();
        if self.look.style == Style::Clay {
            let amount = 0.32 * self.look.texture.unwrap_or(1.0) as f32;
            for (n, p) in g.normals.iter_mut().zip(&g.positions) {
                let q = *p * 65.0;
                let e = 0.05;
                let grad = Vec3::new(
                    img::fbm3(q + Vec3::X * e) - img::fbm3(q - Vec3::X * e),
                    img::fbm3(q + Vec3::Y * e) - img::fbm3(q - Vec3::Y * e),
                    img::fbm3(q + Vec3::Z * e) - img::fbm3(q - Vec3::Z * e),
                ) / (2.0 * e);
                let tangential = grad - *n * n.dot(grad);
                *n = (*n - tangential * amount).normalize_or(*n);
            }
        }
        let mut d = (*mesh.geometry).clone();
        d.colors = (0..d.positions.len()).map(|i| Vec4::new(mask(i), img::fbm3(d.positions[i] * 70.0), 0.0, 1.0)).collect();
        let g = Arc::new(g);
        mesh.geometry = Arc::clone(&g);
        mesh.material = match self.look.style {
            Style::Clay => Material::standard(Rgb::WHITE).roughness(0.9).env_intensity(0.7),
            _ => Material::standard(Rgb::WHITE).roughness(0.52).clearcoat(0.12),
        };
        mesh.material.clearcoat_roughness = 0.45;
        self.dressed.push((g, Arc::new(d)));
        out
    }

    /// An empty scene for the look: transparent, no lights (`render` adds the look's lights
    /// and environment relative to the camera).
    pub fn stage(&self) -> Scene {
        Scene::new().background(Background::Transparent)
    }

    /// Renders one frame. `moment(dt)` builds the scene and camera `dt` frames from the
    /// frame's time; with motion blur it is called once per sub-frame across the shutter.
    pub fn render(&self, moment: impl Fn(f32) -> (Scene, Camera)) -> Pixels {
        let n = self.look.motion_blur.max(1) as usize;
        let shutter = self.look.shutter as f32 / 360.0;
        let layers: Vec<Layer> = (0..n)
            .filter_map(|i| {
                let dt = if n == 1 { 0.0 } else { ((i as f32 + 0.5) / n as f32 - 0.5) * shutter };
                let (scene, camera) = moment(dt);
                self.layer(&scene, &camera)
            })
            .collect();
        let empty = Rect { x0: 0, y0: 0, w: 0, h: 0 };
        match layers.len() {
            0 => img::encode(self.width, self.height, empty, |_, _| [0.0; 4]),
            1 => {
                let l = &layers[0];
                img::encode(self.width, self.height, l.rect, |x, y| l.rgba[y * l.rect.w + x])
            }
            k => {
                let (w, h) = (self.width as usize, self.height as usize);
                let mut sum = vec![[0f32; 4]; w * h];
                for l in &layers {
                    l.accumulate(&mut sum, w, 1.0 / n as f32);
                }
                let _ = k;
                let x0 = layers.iter().map(|l| l.rect.x0).min().unwrap();
                let y0 = layers.iter().map(|l| l.rect.y0).min().unwrap();
                let x1 = layers.iter().map(|l| l.rect.x0 + l.rect.w).max().unwrap();
                let y1 = layers.iter().map(|l| l.rect.y0 + l.rect.h).max().unwrap();
                let rect = Rect { x0, y0, w: x1 - x0, h: y1 - y0 };
                img::encode(self.width, self.height, rect, |x, y| sum[(y + y0) * w + x + x0])
            }
        }
    }

    /// One moment through the look's passes and post-process.
    fn layer(&self, scene: &Scene, camera: &Camera) -> Option<Layer> {
        let scale = self.height as f32 / 1080.0;
        let look = &self.look;
        let reach = look.outline as f32 * 0.5 * 1.6 + look.misregister.unwrap_or(3.0) as f32 + 6.0;
        let margin = (reach * scale).ceil() as usize + 2;
        let anchor_world = anchor(scene, camera);
        let projected = camera.project(anchor_world, self.width as f32, self.height as f32).unwrap_or(Vec3::new(self.width as f32 * 0.5, self.height as f32 * 0.5, 1.0));
        let ppm = camera.pixel_size(anchor_world, 1.0, self.height as f32);
        let (forward, right, up) = (camera.forward(), camera.right(), camera.up_vector());
        if self.stylized() {
            let (near, far) = depth_range(scene, camera);
            let fog_far = near + (far - near) / FOG_SPAN;
            let (normals, data) = rayon::join(|| self.flat_pass(scene, camera, None), || self.flat_pass(scene, camera, Some((near, fog_far))));
            let rect = Rect::around(&normals, margin)?;
            let (nrgb, alpha) = img::decode(&normals, rect);
            let (drgb, _) = img::decode(&data, rect);
            // The normals, softened a little inside the alpha (the mesh's facets and bumps
            // would make the band edges ragged).
            let mut ch: Vec<Vec<f32>> = (0..3).map(|k| nrgb.iter().zip(&alpha).map(|(c, a)| (c[k] * 2.0 - 1.0) * a).collect()).collect();
            ch.push(alpha.clone());
            ch.par_iter_mut().for_each(|c| img::blur(c, rect.w, rect.h, 1.6 * scale));
            let normal = (0..rect.len())
                .map(|i| {
                    let n = Vec3::new(ch[0][i], ch[1][i], ch[2][i]).normalize_or(Vec3::Z);
                    Vec3::new(n.dot(right), n.dot(up), -n.dot(forward))
                })
                .collect();
            let fade = |c: &Vec3| (1.0 - c.z).max(0.05);
            let inputs = shade::Inputs {
                look,
                rect,
                scale,
                ppm,
                anchor: Vec2::new(projected.x - rect.x0 as f32, projected.y - rect.y0 as f32),
                alpha,
                color: Vec::new(),
                normal,
                nail: drgb.iter().map(|c| (c.x / fade(c)).clamp(0.0, 1.0)).collect(),
                noise: drgb.iter().map(|c| (c.y / fade(c)).clamp(0.0, 1.0)).collect(),
                depth: drgb.iter().map(|c| near + c.z * (fog_far - near)).collect(),
                brush: brush(),
            };
            Some(shade::stylized(&inputs))
        } else {
            let px = self.color_pass(scene, camera);
            let rect = Rect::around(&px, margin)?;
            let (color, alpha) = img::decode(&px, rect);
            let inputs = shade::Inputs {
                look,
                rect,
                scale,
                ppm,
                anchor: Vec2::new(projected.x - rect.x0 as f32, projected.y - rect.y0 as f32),
                alpha,
                color,
                normal: Vec::new(),
                nail: Vec::new(),
                noise: Vec::new(),
                depth: Vec::new(),
                brush: None,
            };
            Some(match look.style {
                Style::Clay => shade::clay(&inputs),
                _ => shade::realistic(&inputs),
            })
        }
    }

    /// The scene lit for the look and rendered in colour.
    fn color_pass(&self, scene: &Scene, camera: &Camera) -> Pixels {
        let mut s = scene.clone();
        s.background = Background::Transparent;
        self.light(&mut s, camera);
        self.color.render_pixels(&s, camera)
    }

    /// Adds the look's lights and environment, placed relative to the camera.
    pub fn light(&self, scene: &mut Scene, camera: &Camera) {
        let (right, up, back) = (camera.right(), camera.up_vector(), -camera.forward());
        let dir = |x: f32, y: f32, z: f32| (right * x + up * y + back * z).normalize();
        let azimuth = back.x.atan2(back.z);
        let soft = Shadow { softness: 4.0, strength: 0.85, bias: 4.0, ..Default::default() };
        match self.look.style {
            Style::Clay => {
                scene.environment = Environment::studio().intensity(0.55).rotated(azimuth);
                scene.add(Node::light(Light::hemisphere(Rgb::hex(0xfffaf4), Rgb::hex(0x5a5048), 1.25)));
                scene.add(Node::light(Light::sun(Rgb::hex(0xfff0e2), 2.2, dir(-0.45, 0.7, 0.65)).with_shadow(Shadow { softness: 6.0, strength: 0.6, ..soft })));
                scene.add(Node::light(Light::directional(Rgb::hex(0xe6ecff), 0.5, dir(0.8, 0.1, 0.5))));
            }
            _ => {
                scene.environment = Environment::studio().intensity(0.6).rotated(azimuth);
                scene.add(Node::light(Light::hemisphere(Rgb::hex(0xfff1e6), Rgb::hex(0x4a3c36), 0.85)));
                scene.add(Node::light(Light::sun(Rgb::hex(0xffe4cc), 2.4, dir(-0.5, 0.7, 0.6)).with_shadow(soft)));
                scene.add(Node::light(Light::directional(Rgb::hex(0xd2e2ff), 1.6, dir(0.8, 0.35, -0.55))));
            }
        }
    }

    /// The normals pass, or with `fog` the data pass: unlit vertex colours (R nail mask, G
    /// object-space noise) fading to blue with the distance from the camera.
    fn flat_pass(&self, scene: &Scene, camera: &Camera, fog: Option<(f32, f32)>) -> Pixels {
        let mut s = scene.clone();
        s.background = Background::Transparent;
        s.environment = Environment::None;
        s.reflector = None;
        s.fog = fog.map(|(near, far)| Fog::Linear { color: Rgb::linear(0.0, 0.0, 1.0), near, far });
        let ids: Vec<NodeId> = s.nodes().map(|(id, _)| id).collect();
        for id in ids {
            let node = &mut s[id];
            if node.light_mut().is_some() {
                node.visible = false;
                continue;
            }
            let Some(mesh) = node.mesh_mut() else { continue };
            let m = &mesh.material;
            if m.shading == Shading::ShadowCatcher || m.opacity < 1.0 || m.transparent {
                node.visible = false;
                continue;
            }
            mesh.cast_shadow = false;
            mesh.material = match fog {
                None => Material::normals(),
                Some(_) => match self.dressed.iter().find(|(c, _)| Arc::ptr_eq(c, &mesh.geometry)) {
                    Some((_, data)) => {
                        mesh.geometry = Arc::clone(data);
                        Material::unlit(Rgb::WHITE)
                    }
                    None => Material::unlit(Rgb::BLACK),
                },
            };
        }
        self.flat.render_pixels(&s, camera)
    }
}

/// The point the screen-space textures follow: the first hand's `hand` bone, else the
/// camera's target.
fn anchor(scene: &Scene, camera: &Camera) -> Vec3 {
    ["hand_R", "hand_L"].iter().find_map(|n| scene.find(n)).map_or(camera.target, |id| scene.world_position(id))
}

/// The distances from the camera that the skinned meshes span.
fn depth_range(scene: &Scene, camera: &Camera) -> (f32, f32) {
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    for (_, node) in scene.nodes() {
        let Some(skin) = node.as_mesh().and_then(|m| m.skin.as_ref()) else { continue };
        for j in &skin.joints {
            let d = (scene.world_position(*j) - camera.position).length();
            lo = lo.min(d);
            hi = hi.max(d);
        }
    }
    if lo > hi {
        let d = (camera.target - camera.position).length();
        (lo, hi) = (d - 0.3, d + 0.3);
    }
    ((lo - 0.06).max(camera.near), hi + 0.06)
}
