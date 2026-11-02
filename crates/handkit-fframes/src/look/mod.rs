//! Looks: how the hands are drawn (`"look"` in a pose or clip, see `handkit_core::look`).
//!
//! A [`Styler`] dresses the hand models for a look, lights the scene relative to the camera,
//! renders the passes a look needs on the CPU and post-processes them:
//!
//! - realistic: the baked skin maps (normal, roughness, occlusion) and a colour map for the
//!   look's skin tone, rebuilt from the tone-free masks (`surface`), one lit colour pass, then
//!   screen-space subsurface scattering and warm shadows;
//! - clay: one lit colour pass of matte clay with lumps and fingerprints in a normal map;
//! - toon, spiderverse, arcane: a normals pass (`Material::normals`, with arcane's brush
//!   strokes in a normal map) and a data pass (nails, creases, palm or brush colour from a map
//!   in the atlas, and depth through fog), shaded in screen space: cel bands from N·L, ink
//!   lines from alpha, normal and depth edges and the main creases, halftone.
//!
//! Everything textured is in the hand's UV atlas, so it moves with the skin; only spiderverse's
//! halftone dots stay on the screen, as print does.
//!
//! A look's `wall` renders one more pass under the hands, whatever the style: a wall facing the
//! camera that catches the shadows a lamp throws, the hands drawn flat in the wall's colour so
//! they cast shadows but never show through (a fading arm stays faded). The lamp also lights
//! the hands in the lit looks, in a dim room.
//!
//! The look's `arm` ends the forearm at the elbow (`arm.rs`): a fading arm renders one more flat
//! pass, each vertex's opacity, and multiplies every layer by it after the post-process (ink
//! included); a sleeve is a cloth mesh on the forearm joint, lit as matte cloth in the lit looks
//! and shaded in its own cel palette in the stylized ones, its end past the elbow fading out
//! the same way.
//!
//! ```rust,ignore
//! let mut styler = Styler::new(Look::from_value(&look_json)?, 1920, 1080);
//! let hands: Vec<HandModel> = models.iter().map(|m| styler.dress(m)).collect();
//! // per frame: the scene and camera `dt` frames from the frame's time (motion blur)
//! let image = styler.render(|dt| build_scene_at(t + dt / fps)).into_image();
//! ```

mod arm;
mod gpu;
mod img;
mod shade;
mod surface;

pub use handkit_core::look::{Arm, Look, Style, TONES, Wall};

use std::sync::Arc;

use fframes_three_d::{
    Aabb, Background, Camera, Environment, Fog, Geometry, Light, Material, Node, NodeId, Pixels, Quat, Renderer, Rgb, Scene, Shading, Shadow,
    Texture, ToneMapping, Vec2, Vec3, Vec4,
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
    /// The dressed hands (and sleeves).
    dressed: Vec<Dressed>,
    /// The colours the hands are dressed in, one entry per person's tone or sleeve.
    tones: Vec<Paint>,
}

/// A colour the stylized looks shade in: a person's skin and nails, or a sleeve's cloth (sRGB).
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Paint {
    pub skin: [u8; 3],
    pub nail: [u8; 3],
    pub cloth: bool,
}

/// What the flat passes draw for a dressed hand or sleeve.
struct Dressed {
    /// The geometry of the colour pass, which identifies the hand in a scene.
    color: Arc<Geometry>,
    /// The data pass: its geometry (vertex colours without a data map) and data map.
    data: Arc<Geometry>,
    data_map: Option<Arc<Texture>>,
    /// The normals pass' normal map (arcane's strokes).
    normal_map: Option<Arc<Texture>>,
    /// Its skin and nail colours: an index in `Styler::tones`.
    tone: usize,
    /// The geometry without vertex colours, for the tone pass.
    plain: Arc<Geometry>,
    /// A fading arm (or a sleeve's end): the geometry with each vertex's opacity in its colours.
    fade: Option<Arc<Geometry>>,
}

/// The flat passes of the stylized looks.
#[derive(Clone, Copy)]
enum Flat {
    /// View normals (with the normal map).
    Normals,
    /// The data map, fading to blue from near to far.
    Data(f32, f32),
    /// Each hand in its skin colour, unlit: which tone a pixel shows, with more than one.
    Tone,
    /// Each hand's opacity (a fading arm, a sleeve's end), unlit; everything else opaque.
    Fade,
}

/// Fog runs from near to this fraction of the data pass's depth range, so the colour channels
/// it fades stay readable.
const FOG_SPAN: f32 = 0.6;

/// Motion blur: the farthest apart (in pixels of a 1080-high frame) the sub-frame copies of a
/// moving joint may land before more sub-frames are rendered, and how many times the look's
/// own count that may grow to.
const BLUR_STEP_PX: f32 = 6.0;
const BLUR_MAX_SCALE: usize = 6;

impl Styler {
    pub fn new(look: Look, width: u32, height: u32) -> Self {
        let color = Renderer::new(width, height).gpu().tone_mapping(ToneMapping::Neutral).exposure(match look.style {
            Style::Clay => 1.05,
            _ => 1.0,
        });
        let flat = Renderer::new(width, height).gpu().tone_mapping(ToneMapping::Linear);
        Styler { look, width, height, color, flat, dressed: Vec::new(), tones: Vec::new() }
    }

    fn stylized(&self) -> bool {
        matches!(self.look.style, Style::Toon | Style::Spiderverse | Style::Arcane)
    }

    /// A copy of `model` dressed for the look: its material, the colour map of the skin tone
    /// (realistic), the maps of the stylized passes and clay. Textures are built once per hand,
    /// look and tone, and shared. Add the dressed model to the scenes you render.
    pub fn dress(&mut self, model: &HandModel) -> HandModel {
        let own = self.look.clone();
        self.dress_as(model, &own)
    }

    /// `dress` in another skin and nail colour (sRGB; no nail: the one the style gives the
    /// skin), with the look's arm.
    pub fn dress_in(&mut self, model: &HandModel, skin: [u8; 3], nail: Option<[u8; 3]>) -> HandModel {
        let own = Look { skin, nail, ..self.look.clone() };
        self.dress_as(model, &own)
    }

    /// `dress` in a person's look (`Look::for_person`): its skin, nail and arm; the style and
    /// everything else are this look's. Every style shows them: realistic builds a colour map
    /// for each tone, the stylized looks shade each hand (and sleeve) in its own palette.
    pub fn dress_as(&mut self, model: &HandModel, own: &Look) -> HandModel {
        let (skin_rgb, nail) = (own.skin, own.nail);
        let mut out = model.clone();
        let Some(id) = out.mesh() else { return out };
        let style = self.look.style;
        let mesh = out.model.scene[id].mesh_mut().unwrap();
        let loaded = mesh.material.clone();
        let nail_rgb = nail.unwrap_or_else(|| match style {
            // Realistic nails follow the tone's own nail colour (lighter on darker skin).
            Style::Realistic => handkit_mesh::skin::nail_color(handkit_mesh::skin::linear(skin_rgb)).map(|c| (img::linear_to_srgb(c) * 255.0).round() as u8),
            _ => Look { skin: skin_rgb, nail: None, ..self.look.clone() }.nail_color(),
        });
        let skin = img::srgb(skin_rgb);
        let nail = img::srgb(nail_rgb);
        let mask = |i: usize| model.nail.get(i).copied().unwrap_or(0.0);
        let mut g = (*mesh.geometry).clone();
        let textured = |kind| surface::texture(model, kind);
        mesh.material = match style {
            Style::Realistic => match textured(surface::Kind::Tone(skin_rgb, nail_rgb)) {
                Some(map) => {
                    g.colors.clear();
                    let mut m = Material::standard(Rgb::WHITE).roughness(1.0).clearcoat(0.1).map(map);
                    m.normal_map = loaded.normal_map.clone();
                    m.metal_rough_map = loaded.metal_rough_map.clone();
                    m.ao_map = loaded.ao_map.clone();
                    m
                }
                None => {
                    // No baked maps: the tone per vertex, a little desaturated (lit skin reads
                    // more saturated than its albedo, darker tones the most).
                    let lum = skin.dot(Vec3::new(0.2126, 0.7152, 0.0722));
                    let body = skin.lerp(Vec3::splat(lum), (0.18 + 0.25 * (1.0 - lum / 0.6)).clamp(0.18, 0.43)) * Vec3::new(1.02, 0.97, 1.0);
                    g.colors = (0..g.positions.len()).map(|i| body.lerp(nail, mask(i)).extend(1.0)).collect();
                    Material::standard(Rgb::WHITE).roughness(0.52).clearcoat(0.12)
                }
            },
            Style::Clay => {
                // The skin's colour as a matte, slightly more saturated body; nails barely show.
                let body = skin * Vec3::new(1.04, 0.93, 0.8);
                g.colors = (0..g.positions.len()).map(|i| body.lerp(nail, mask(i) * 0.3).extend(1.0)).collect();
                let mut m = Material::standard(Rgb::WHITE).roughness(0.9).env_intensity(0.7);
                match textured(surface::Kind::ClayNormal) {
                    Some(map) => m.normal_map = Some(map),
                    None => lumpy_normals(&mut g, 0.32),
                }
                m.normal_scale = self.look.texture.unwrap_or(1.0) as f32;
                m
            }
            _ => Material::standard(Rgb::WHITE).roughness(0.52),
        };
        mesh.material.clearcoat_roughness = 0.45;
        let (data_map, normal_map) = match style {
            Style::Toon | Style::Spiderverse => (textured(surface::Kind::Data), None),
            Style::Arcane => (textured(surface::Kind::ArcaneData), textured(surface::Kind::ArcaneNormal)),
            _ => (None, None),
        };
        let mut d = (*mesh.geometry).clone();
        // Without a data map, the data rides in vertex colours: R nails, G object noise.
        d.colors = match data_map {
            Some(_) => Vec::new(),
            None => (0..d.positions.len())
                .map(|i| {
                    let g = if style == Style::Arcane { img::fbm3(d.positions[i] * 70.0) } else { 0.0 };
                    Vec4::new(surface::pack_nail(mask(i)), g, 0.0, 1.0)
                })
                .collect(),
        };
        // A fading arm: each vertex's opacity, for the fade pass.
        let fade = match (own.arm, arm::Forearm::of(model)) {
            (Arm::Fade, Some(forearm)) => {
                let mut f = g.clone();
                f.colors = f.positions.iter().map(|p| Vec3::splat(forearm.fade(*p)).extend(1.0)).collect();
                Some(Arc::new(f))
            }
            _ => None,
        };
        let g = Arc::new(g);
        mesh.geometry = Arc::clone(&g);
        let tone = self.tone_of(Paint { skin: skin_rgb, nail: nail_rgb, cloth: false });
        let d = Arc::new(d);
        let plain = match (d.colors.is_empty(), g.colors.is_empty()) {
            (true, _) => Arc::clone(&d),
            (_, true) => Arc::clone(&g),
            _ => {
                let mut p = (*g).clone();
                p.colors.clear();
                Arc::new(p)
            }
        };
        self.dressed.push(Dressed { color: g, data: d, data_map, normal_map, tone, plain, fade });
        if let Arm::Sleeve { color, length } = own.arm {
            self.add_sleeve(&mut out, model, color, length);
        }
        out
    }

    /// The index of a paint in `tones`, added when new.
    fn tone_of(&mut self, paint: Paint) -> usize {
        self.tones.iter().position(|t| *t == paint).unwrap_or_else(|| {
            self.tones.push(paint);
            self.tones.len() - 1
        })
    }

    /// Puts a rolled-up sleeve of `color` (sRGB) on the dressed hand's forearm joint, `length`
    /// meters from the elbow: matte cloth, with no nails, creases or palm in the data pass, its
    /// end past the elbow fading out in the fade pass.
    fn add_sleeve(&mut self, out: &mut HandModel, model: &HandModel, color: [u8; 3], length: f64) {
        let (Some(forearm), Some(mesh)) = (arm::Forearm::of(model), model.mesh()) else { return };
        let Some(skin) = model.model.scene[mesh].as_mesh() else { return };
        let (g, opacity) = forearm.sleeve(&skin.geometry.positions, length as f32);
        let mut fade = g.clone();
        fade.colors = opacity.iter().map(|a| Vec3::splat(*a).extend(1.0)).collect();
        let mut d = g.clone();
        let arcane = self.look.style == Style::Arcane;
        d.colors = d.positions.iter().map(|p| Vec4::new(surface::pack_nail(0.0), if arcane { img::fbm3(*p * 70.0) } else { 0.0 }, 0.0, 1.0)).collect();
        let c = img::srgb(color);
        let material = Material::standard(Rgb::linear(c.x, c.y, c.z)).roughness(1.0).env_intensity(0.6);
        let g = Arc::new(g);
        out.model.scene.add_to(forearm.joint, Node::mesh(Arc::clone(&g), material).named("sleeve"));
        let tone = self.tone_of(Paint { skin: color, nail: color, cloth: true });
        self.dressed.push(Dressed { color: Arc::clone(&g), data: Arc::new(d), data_map: None, normal_map: None, tone, plain: g, fade: Some(Arc::new(fade)) });
    }

    /// An empty scene for the look: transparent, no lights (`render` adds the look's lights
    /// and environment relative to the camera).
    pub fn stage(&self) -> Scene {
        Scene::new().background(Background::Transparent)
    }

    /// Renders one frame. `moment(dt)` builds the scene and camera `dt` frames from the
    /// frame's time; with motion blur it is called once per sub-frame across the shutter.
    pub fn render(&self, moment: impl Fn(f32) -> (Scene, Camera)) -> Pixels {
        let shutter = self.look.shutter as f32 / 360.0;
        let n = self.sub_frames(&moment, shutter);
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

    /// How many sub-frames to average: the look's `motion_blur`, raised for a fast move so the
    /// copies across the shutter sit at most `BLUR_STEP_PX` apart on screen (a smear, not a
    /// row of ghosts), up to `BLUR_MAX_SCALE` times the look's count.
    fn sub_frames(&self, moment: &impl Fn(f32) -> (Scene, Camera), shutter: f32) -> usize {
        let n = self.look.motion_blur.max(1) as usize;
        if n == 1 {
            return 1;
        }
        // Every skinned joint on screen, at the shutter's open and close.
        let at = |dt: f32| -> Vec<Option<Vec3>> {
            let (scene, camera) = moment(dt);
            let joints = scene.nodes().filter_map(|(_, node)| node.as_mesh().and_then(|m| m.skin.as_ref())).flat_map(|skin| skin.joints.clone());
            joints.map(|j| camera.project(scene.world_position(j), self.width as f32, self.height as f32)).collect()
        };
        let (a, b) = (at(-0.5 * shutter), at(0.5 * shutter));
        let travel = a.iter().zip(&b).filter_map(|(p, q)| Some((p.as_ref()?.truncate() - q.as_ref()?.truncate()).length())).fold(0.0, f32::max) / (self.height as f32 / 1080.0);
        n.max((travel / BLUR_STEP_PX).ceil() as usize).min(n * BLUR_MAX_SCALE)
    }

    /// One moment through the look's passes and post-process, over the wall if it has one
    /// (which shows even with no hand in view).
    fn layer(&self, scene: &Scene, camera: &Camera) -> Option<Layer> {
        let hands = self.hands_layer(scene, camera);
        match self.look.wall {
            Some(wall) => Some(self.on_wall(hands, self.wall_pass(scene, camera, &wall))),
            None => hands,
        }
    }

    /// The hands through the look's passes and post-process; none when none is in view.
    fn hands_layer(&self, scene: &Scene, camera: &Camera) -> Option<Layer> {
        let scale = self.height as f32 / 1080.0;
        let look = &self.look;
        let reach = look.outline as f32 * 0.5 * 1.6 + look.misregister.unwrap_or(3.0) as f32 + 6.0;
        let margin = (reach * scale).ceil() as usize + 2;
        let anchor_world = anchor(scene, camera);
        let projected = camera.project(anchor_world, self.width as f32, self.height as f32).unwrap_or(Vec3::new(self.width as f32 * 0.5, self.height as f32 * 0.5, 1.0));
        let ppm = camera.pixel_size(anchor_world, 1.0, self.height as f32);
        let (forward, right, up) = (camera.forward(), camera.right(), camera.up_vector());
        let fades = self.dressed.iter().any(|d| d.fade.is_some());
        // A fading arm or sleeve: every layer times each pixel's opacity, carried out past the
        // silhouette so the ink and plates around a faded forearm fade with it.
        let faded = |layer: Layer, px: Option<Pixels>| -> Layer {
            let Some(px) = px else { return layer };
            let Rect { w, h, .. } = layer.rect;
            let (rgb, a) = img::decode(&px, layer.rect);
            let mut num: Vec<f32> = rgb.iter().zip(&a).map(|(c, a)| c.x * a).collect();
            let mut den = a.clone();
            let sigma = margin as f32 * 0.5;
            rayon::join(|| img::blur(&mut num, w, h, sigma), || img::blur(&mut den, w, h, sigma));
            let rgba = (layer.rgba.par_iter(), rgb.par_iter(), a.par_iter(), num.par_iter(), den.par_iter())
                .into_par_iter()
                .map(|(p, c, a, num, den)| {
                    let around = if *den > 1e-4 { num / den } else { 1.0 };
                    let m = c.x * a + around * (1.0 - a);
                    p.map(|v| v * m)
                })
                .collect();
            Layer { rect: layer.rect, rgba }
        };
        if self.stylized() {
            let (near, far) = depth_range(scene, camera);
            let fog_far = near + (far - near) / FOG_SPAN;
            let several = self.tones.len() > 1;
            let ((normals, opacity), (data, tones)) = rayon::join(
                || rayon::join(|| self.flat_pass(scene, camera, Flat::Normals), || fades.then(|| self.flat_pass(scene, camera, Flat::Fade))),
                || rayon::join(|| self.flat_pass(scene, camera, Flat::Data(near, fog_far)), || several.then(|| self.flat_pass(scene, camera, Flat::Tone))),
            );
            let rect = Rect::around(&normals, margin)?;
            let (nrgb, alpha) = img::decode(&normals, rect);
            let (drgb, _) = img::decode(&data, rect);
            // Each pixel's tone: the nearest of the hands' skin colours.
            let tone: Vec<u8> = match &tones {
                Some(px) => {
                    let skins: Vec<Vec3> = self.tones.iter().map(|p| img::srgb(p.skin)).collect();
                    img::decode(px, rect)
                        .0
                        .par_iter()
                        .map(|c| (0..skins.len()).min_by(|a, b| (skins[*a] - *c).length_squared().total_cmp(&(skins[*b] - *c).length_squared())).unwrap_or(0) as u8)
                        .collect()
                }
                None => Vec::new(),
            };
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
                nail: Vec::new(),
                cavity: Vec::new(),
                detail: drgb.iter().map(|c| (c.y / fade(c)).clamp(0.0, 1.0)).collect(),
                depth: drgb.iter().map(|c| near + c.z * (fog_far - near)).collect(),
                tones: &self.tones,
                tone,
            };
            let (nail, cavity): (Vec<f32>, Vec<f32>) = drgb.iter().map(|c| surface::unpack((c.x / fade(c)).clamp(0.0, 1.0))).unzip();
            let inputs = shade::Inputs { nail, cavity, ..inputs };
            Some(faded(shade::stylized(&inputs), opacity))
        } else {
            let (px, opacity) = rayon::join(|| self.color_pass(scene, camera), || fades.then(|| self.flat_pass(scene, camera, Flat::Fade)));
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
                cavity: Vec::new(),
                detail: Vec::new(),
                depth: Vec::new(),
                tones: &self.tones,
                tone: Vec::new(),
            };
            let layer = match look.style {
                Style::Clay => shade::clay(&inputs),
                _ => shade::realistic(&inputs),
            };
            Some(faded(layer, opacity))
        }
    }

    /// `hands` over the wall (the whole frame).
    fn on_wall(&self, hands: Option<Layer>, mut out: Layer) -> Layer {
        let Some(hands) = hands else { return out };
        let w = self.width as usize;
        out.rgba.par_chunks_mut(w).skip(hands.rect.y0).take(hands.rect.h).enumerate().for_each(|(y, row)| {
            let src = &hands.rgba[y * hands.rect.w..(y + 1) * hands.rect.w];
            for (d, s) in row[hands.rect.x0..hands.rect.x0 + hands.rect.w].iter_mut().zip(src) {
                let keep = 1.0 - s[3];
                *d = [s[0] + d[0] * keep, s[1] + d[1] * keep, s[2] + d[2] * keep, 1.0];
            }
        });
        out
    }

    /// Where the wall's lamp stands: given, or beside the camera, up and to the left.
    fn lamp(wall: &Wall, camera: &Camera) -> Vec3 {
        match wall.lamp {
            Some([x, y, z]) => Vec3::new(x as f32, y as f32, z as f32),
            None => {
                let d = (camera.target - camera.position).length();
                camera.position - camera.right() * 0.7 * d + camera.up_vector() * 0.3 * d
            }
        }
    }

    /// The wall, lit by its lamp with the hands' shadows on it: the whole frame, opaque.
    fn wall_pass(&self, scene: &Scene, camera: &Camera, wall: &Wall) -> Layer {
        let mut s = scene.clone();
        s.background = Background::Transparent;
        s.environment = Environment::None;
        s.reflector = None;
        s.fog = None;
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
            // White: no shadow where the hands cover the wall.
            mesh.material = Material::unlit(Rgb::WHITE);
            mesh.cast_shadow = true;
        }
        let forward = camera.forward();
        let v3 = |a: [f64; 3]| Vec3::new(a[0] as f32, a[1] as f32, a[2] as f32);
        // The wall's plane: fixed in the world, or facing the camera behind what it looks at.
        let (point, normal) = match wall.at {
            Some(at) => (v3(at), v3(wall.facing).normalize_or(Vec3::Z)),
            None => (camera.target + forward * wall.behind as f32, -forward),
        };
        let meet = |from: Vec3, along: Vec3| {
            let across = along.dot(normal);
            let t = (point - from).dot(normal) / across;
            (across.abs() > 1e-5 && t > 0.0).then(|| from + along * t)
        };
        let center = meet(camera.position, forward).unwrap_or(point);
        let depth = (center - camera.position).length();
        let aspect = self.width as f32 / self.height as f32;
        // The wall in view (more of it when seen at a slant), and a margin.
        let slant = forward.dot(normal).abs().max(0.25);
        let h = (2.0 * depth * (camera.fov.to_radians() * 0.5).tan() + 2.0 * camera.shift.length() * depth) / slant;
        let side = 1.4 * h * aspect.max(1.0);
        s.add(Node::mesh(Arc::new(Geometry::plane(side, side)), Material::shadow_catcher(1.0)).with_position(center).with_rotation(Quat::from_rotation_arc(Vec3::Z, normal)));
        let lamp = Self::lamp(wall, camera);
        // A larger lamp blurs the edges more; the map is sized for crisp edges at a bare bulb.
        // The map covers the wall in view and the space in front of it, where the hands are
        // (fitted to the hands alone, it would end before the wall).
        let u = (camera.right() - normal * camera.right().dot(normal)).normalize_or(normal.any_orthonormal_vector());
        let w = normal.cross(u);
        let (hw, hh) = (0.5 * h * aspect, 0.5 * h);
        let corners = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(x, y)| center + u * x * hw + w * y * hh);
        let front = normal * ((camera.target - point).dot(normal).abs() + 0.2);
        let bounds = Aabb::from_points(corners.iter().flat_map(|c| [*c, *c + front]));
        let shadow = Shadow { map_size: 4096, softness: 0.8 + 10.0 * wall.softness as f32, strength: 1.0, bias: 1.5, bounds: Some(bounds) };
        s.add(Node::light(Light::directional(Rgb::WHITE, 1.0, lamp - camera.target).with_shadow(shadow)));
        let px = gpu::render_pixels(&self.flat, s, camera);
        let full = Rect { x0: 0, y0: 0, w: self.width as usize, h: self.height as usize };
        let (rgb, alpha) = img::decode(&px, full);
        // The lamp's pool of light: centred where its ray through what the camera looks at
        // meets the wall, falling off toward the frame's corners.
        let hit = meet(camera.target, camera.target - lamp).unwrap_or(center);
        let pool = camera.project(hit, self.width as f32, self.height as f32).map(|p| Vec2::new(p.x, p.y)).unwrap_or(Vec2::new(self.width as f32, self.height as f32) * 0.5);
        let c = img::srgb(wall.color);
        let hgt = self.height as f32;
        let rgba = (0..full.len())
            .into_par_iter()
            .map(|i| {
                let (x, y) = ((i % full.w) as f32, (i / full.w) as f32);
                let r = (Vec2::new(x, y) - pool).length() / hgt;
                let light = 1.0 - 0.55 * img::smoothstep(0.25, 1.1, r);
                let shade = alpha[i] * (1.0 - rgb[i].x);
                let v = c * light * (1.0 - 0.86 * shade);
                [v.x, v.y, v.z, 1.0]
            })
            .collect();
        Layer { rect: full, rgba }
    }

    /// The scene lit for the look and rendered in colour.
    fn color_pass(&self, scene: &Scene, camera: &Camera) -> Pixels {
        let mut s = scene.clone();
        s.background = Background::Transparent;
        self.light(&mut s, camera);
        gpu::render_pixels(&self.color, s, camera)
    }

    /// Adds the look's lights and environment, placed relative to the camera.
    pub fn light(&self, scene: &mut Scene, camera: &Camera) {
        let (right, up, back) = (camera.right(), camera.up_vector(), -camera.forward());
        let dir = |x: f32, y: f32, z: f32| (right * x + up * y + back * z).normalize();
        let azimuth = back.x.atan2(back.z);
        let soft = Shadow { softness: 4.0, strength: 0.85, bias: 4.0, ..Default::default() };
        if let Some(wall) = self.look.wall {
            // A dim room lit by the lamp, which throws the hands' shadows on each other too.
            let from = Self::lamp(&wall, camera) - camera.target;
            scene.environment = Environment::studio().intensity(0.25).rotated(azimuth);
            scene.add(Node::light(Light::hemisphere(Rgb::hex(0xfff4e8), Rgb::hex(0x302a26), 0.45)));
            scene.add(Node::light(Light::sun(Rgb::hex(0xfff1dc), 3.0, from).with_shadow(Shadow { softness: 2.0 + 6.0 * wall.softness as f32, strength: 0.9, ..soft })));
            scene.add(Node::light(Light::directional(Rgb::hex(0xe8f0ff), 0.35, dir(0.8, 0.35, -0.55))));
            return;
        }
        match self.look.style {
            Style::Clay => {
                scene.environment = Environment::studio().intensity(0.55).rotated(azimuth);
                scene.add(Node::light(Light::hemisphere(Rgb::hex(0xfffaf4), Rgb::hex(0x5a5048), 1.25)));
                scene.add(Node::light(Light::sun(Rgb::hex(0xfff0e2), 2.2, dir(-0.45, 0.7, 0.65)).with_shadow(Shadow { softness: 6.0, strength: 0.6, ..soft })));
                scene.add(Node::light(Light::directional(Rgb::hex(0xe6ecff), 0.5, dir(0.8, 0.1, 0.5))));
            }
            _ => {
                scene.environment = Environment::studio().intensity(0.6).rotated(azimuth);
                scene.add(Node::light(Light::hemisphere(Rgb::hex(0xffffff), Rgb::hex(0x505055), 1.0)));
                scene.add(Node::light(Light::sun(Rgb::hex(0xfff6ee), 2.6, dir(-0.5, 0.7, 0.6)).with_shadow(soft)));
                scene.add(Node::light(Light::directional(Rgb::hex(0xe8f0ff), 1.2, dir(0.8, 0.35, -0.55))));
            }
        }
    }

    /// A flat pass: the normals, the data map (or vertex colours) unlit fading to blue with
    /// the distance from the camera, each hand's skin colour, or its opacity.
    fn flat_pass(&self, scene: &Scene, camera: &Camera, pass: Flat) -> Pixels {
        let mut s = scene.clone();
        s.background = Background::Transparent;
        s.environment = Environment::None;
        s.reflector = None;
        s.fog = match pass {
            Flat::Data(near, far) => Some(Fog::Linear { color: Rgb::linear(0.0, 0.0, 1.0), near, far }),
            _ => None,
        };
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
            let dressed = self.dressed.iter().find(|d| Arc::ptr_eq(&d.color, &mesh.geometry));
            mesh.material = match (pass, dressed) {
                (Flat::Normals, Some(d)) => {
                    let mut m = Material::normals();
                    m.normal_map = d.normal_map.clone();
                    m.normal_scale = self.look.texture.unwrap_or(1.0) as f32;
                    m
                }
                (Flat::Normals, None) => Material::normals(),
                (Flat::Data(..), Some(d)) => {
                    mesh.geometry = Arc::clone(&d.data);
                    let m = Material::unlit(Rgb::WHITE);
                    match &d.data_map {
                        Some(map) => m.map(Arc::clone(map)),
                        None => m,
                    }
                }
                (Flat::Data(..), None) => Material::unlit(Rgb::BLACK),
                (Flat::Tone, d) => {
                    if let Some(d) = d {
                        mesh.geometry = Arc::clone(&d.plain);
                    }
                    let c = img::srgb(self.tones[d.map_or(0, |d| d.tone)].skin);
                    Material::unlit(Rgb::linear(c.x, c.y, c.z))
                }
                (Flat::Fade, d) => {
                    match d {
                        Some(d) => mesh.geometry = Arc::clone(d.fade.as_ref().unwrap_or(&d.plain)),
                        None if !mesh.geometry.colors.is_empty() => {
                            let mut g = (*mesh.geometry).clone();
                            g.colors.clear();
                            mesh.geometry = Arc::new(g);
                        }
                        None => {}
                    }
                    Material::unlit(Rgb::WHITE)
                }
            };
        }
        gpu::render_pixels(&self.flat, s, camera)
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

/// Lumpy normals from object-space noise, for clay on a hand without UVs.
fn lumpy_normals(g: &mut Geometry, amount: f32) {
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
