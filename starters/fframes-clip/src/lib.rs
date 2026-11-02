//! Plays a Handkit clip (keyed hand poses, `handkit check` / `handkit sheet` to review it) as
//! an fframes video: SVG background, the hands rendered in 3D on top.
//!
//! Copy this folder for a video and restyle `render_frame`: the background, a spotlight,
//! titles. The hands come from `clip.sample(t)`, a pure function of time, so frames can
//! render in any order on any thread.
use fframes::{AudioMap, Color, Duration, FFramesContext, Frame, Svgr, Video, include_media_dir};
use handkit_core::clip::Clip;
use handkit_core::pose::Side;
use handkit_fframes::HandModel;
use handkit_fframes::three_d::*;

include_media_dir!(pub struct FframesClipMedia, "media");

#[cfg(not(feature = "portrait"))]
pub const WIDTH: usize = 1920;
#[cfg(not(feature = "portrait"))]
pub const HEIGHT: usize = 1080;
#[cfg(feature = "portrait")]
pub const WIDTH: usize = 1080;
#[cfg(feature = "portrait")]
pub const HEIGHT: usize = 1920;

pub struct FframesClipVideo<'a> {
    pub media: &'a FframesClipMedia,
    clip: Clip,
    hands: Vec<HandModel>,
    stage: Scene,
    renderer: Renderer,
    background: String,
}

impl<'a> FframesClipVideo<'a> {
    /// `clip` is the clip file's text; the hand meshes come from `assets` (the `.glb` files
    /// `handkit mesh` writes) or are generated (about a second each).
    pub fn new(media: &'a FframesClipMedia, clip: &str, assets: Option<&std::path::Path>, background: &str) -> Result<Self, String> {
        let clip = Clip::parse(clip)?;
        let first = clip.sample(0.0)?;
        let sides: Vec<Side> = [("right", Side::Right), ("left", Side::Left)]
            .into_iter()
            .filter(|(name, _)| first.pose["hands"].get(*name).is_some())
            .map(|(_, side)| side)
            .collect();
        let hands = std::thread::scope(|s| {
            let jobs: Vec<_> = sides
                .iter()
                .map(|side| {
                    s.spawn(move || {
                        let file = assets.map(|dir| dir.join(if *side == Side::Left { "hand_L.glb" } else { "hand_R.glb" }));
                        match file.and_then(|f| std::fs::read(f).ok()) {
                            Some(bytes) => HandModel::from_glb(&bytes).map_err(|e| format!("{e:?}")),
                            None => Ok(HandModel::generate(*side)),
                        }
                    })
                })
                .collect();
            jobs.into_iter().map(|j| j.join().unwrap()).collect::<Result<Vec<_>, String>>()
        })?;
        let mut stage = Scene::new().environment(Environment::studio()).background(Background::Transparent);
        stage.add(Node::light(Light::hemisphere(Rgb::WHITE, Rgb::hex(0x404050), 1.6)));
        stage.add(Node::light(Light::directional(Rgb::WHITE, 2.2, vec3(0.6, 1.0, 0.8))));
        let renderer = Renderer::new(WIDTH as u32, HEIGHT as u32).cpu().tone_mapping(ToneMapping::Neutral);
        Ok(Self { media, clip, hands, stage, renderer, background: background.to_string() })
    }

    pub fn duration_seconds(&self) -> f64 {
        self.clip.duration
    }

    /// The hands at `t` seconds as an image the size of the video (transparent around them).
    fn hands_image(&self, t: f64) -> Option<fframes::media::ImageData<'static>> {
        let frame = self.clip.sample(t).ok()?;
        let solved = handkit_core::solve_value(&frame.pose).ok()?;
        let mut scene = self.stage.clone();
        for model in &self.hands {
            model.add_to(&mut scene).pose(&mut scene, &solved);
        }
        let cam = frame.pose.get("camera").cloned().unwrap_or(serde_json::json!({ "position": [0.0, 0.1, 0.6], "target": [0.0, 0.05, 0.0], "fov": 35 }));
        let v = |k: &str, i: usize| cam[k][i].as_f64().unwrap_or(0.0) as f32;
        let camera = Camera::perspective(cam["fov"].as_f64().unwrap_or(35.0) as f32)
            .at(v("position", 0), v("position", 1), v("position", 2))
            .look_at(vec3(v("target", 0), v("target", 1), v("target", 2)));
        Some(self.renderer.render(&scene, &camera))
    }
}

impl std::fmt::Debug for FframesClipVideo<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FframesClipVideo").field("duration", &self.clip.duration).finish()
    }
}

impl Video for FframesClipVideo<'_> {
    const FPS: usize = 30;
    const WIDTH: usize = WIDTH;
    const HEIGHT: usize = HEIGHT;
    const BACKGROUND_COLOR: Color = Color::BLACK;

    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(self.clip.duration as f32)
    }

    fn audio(&self) -> AudioMap<'_> {
        AudioMap::none()
    }

    fn render_frame<'a>(&'a self, frame: Frame, _ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let hands = self.hands_image(frame.seconds() as f64);
        fframes::svgr!(
            <svg xmlns="http://www.w3.org/2000/svg" viewBox={format!("0 0 {WIDTH} {HEIGHT}")} width={WIDTH} height={HEIGHT}>
                <rect width={WIDTH} height={HEIGHT} fill={self.background.as_str()} />
                {match &hands {
                    Some(image) => fframes::svgr!(<image href={image.href()} x="0" y="0" width={WIDTH} height={HEIGHT} />),
                    None => Svgr::empty(),
                }}
            </svg>
        )
    }
}
