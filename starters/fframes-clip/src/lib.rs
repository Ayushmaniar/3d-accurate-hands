//! Plays a Handkit clip (keyed hand poses, `handkit check` / `handkit sheet` to review it) as
//! an fframes video: SVG background, the hands rendered in 3D on top, in a look (realistic,
//! toon, spiderverse, arcane, clay; see `handkit_fframes::look`).
//!
//! Copy this folder for a video and restyle `render_frame`: the background, a spotlight,
//! titles. The hands come from `clip.sample(t)`, a pure function of time, so frames can
//! render in any order on any thread.
use fframes::{AudioMap, Color, Duration, FFramesContext, Frame, Svgr, Video, include_media_dir};
use handkit_core::clip::Clip;
use handkit_core::pose::Side;
use handkit_fframes::HandModel;
use handkit_fframes::look::{Look, Style, Styler, TONES};
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

const FPS: usize = 30;

/// How the hands are drawn: the clip's `look`, then the flags over it.
#[derive(Debug, Default, Clone)]
pub struct LookArgs {
    /// `--look style[,key=value...]`
    pub look: Option<String>,
    /// `--look-json '{...}'`
    pub look_json: Option<String>,
    /// `--motion-blur N`
    pub motion_blur: Option<u32>,
    /// `--looks "realistic;toon;spiderverse,skin=tan;..."`: a demo that cycles through looks.
    pub looks: Option<String>,
    /// `--look-every S`: seconds per look of `--looks`.
    pub look_every: f64,
}

/// One look, ready to render: its styler and the hands dressed for it.
struct Dressed {
    styler: Styler,
    hands: Vec<HandModel>,
    /// Its name in the looks demo: the style and the skin tone, e.g. "toon · tan".
    label: String,
}

pub struct FframesClipVideo<'a> {
    pub media: &'a FframesClipMedia,
    clip: Clip,
    looks: Vec<Dressed>,
    /// Seconds per look when cycling (`--looks`), with the look's name shown.
    look_every: Option<f64>,
    background: String,
    /// A soft round light behind the hands: centre x as a fraction of the width, y and radius
    /// as fractions of the height, and its color.
    spotlight: Option<(f32, f32, f32, String)>,
}

/// The backdrop each style is shown on in the looks demo.
pub fn backdrop(style: Style) -> &'static str {
    match style {
        Style::Realistic => "#6f7276",
        Style::Toon => "#f1ebdf",
        Style::Spiderverse => "#fbf8f0",
        Style::Arcane => "#161a24",
        Style::Clay => "#bdb8b2",
    }
}

/// The looks to render: one (the clip's look under the flags), or the `--looks` list.
fn looks(clip: &Clip, args: &LookArgs) -> Result<Vec<Look>, String> {
    let flag = match (&args.look, &args.look_json) {
        (Some(_), Some(_)) => return Err("give --look or --look-json, not both".into()),
        (Some(s), None) => Some(Look::parse_flag(s)?),
        (None, Some(j)) => Some(serde_json::from_str(j).map_err(|e| format!("--look-json: {e}"))?),
        (None, None) => None,
    };
    let base = Look::merge(clip.look.as_ref(), flag.as_ref());
    let specs: Vec<Option<serde_json::Value>> = match &args.looks {
        None => vec![base],
        Some(list) => list
            .split(';')
            .filter(|s| !s.trim().is_empty())
            .map(|s| Look::parse_flag(s).map(|v| Look::merge(base.as_ref(), Some(&v))))
            .collect::<Result<_, _>>()?,
    };
    specs
        .into_iter()
        .map(|v| {
            let mut look = v.map_or(Ok(Look::default()), |v| Look::from_value(&v))?;
            if let Some(n) = args.motion_blur {
                look.motion_blur = n.max(1);
            }
            Ok(look)
        })
        .collect()
}

impl<'a> FframesClipVideo<'a> {
    /// `clip` is the clip file's text; the hand meshes come from `assets` (the `.glb` files
    /// `handkit mesh` writes) or are generated (about a second each).
    pub fn new(media: &'a FframesClipMedia, clip: &str, assets: Option<&std::path::Path>, background: &str, spotlight: Option<&str>, look_args: &LookArgs) -> Result<Self, String> {
        let spotlight = match spotlight {
            None => None,
            Some(s) => {
                let p: Vec<&str> = s.split(',').map(str::trim).collect();
                let n = |i: usize| p.get(i).and_then(|x| x.parse::<f32>().ok());
                match (n(0), n(1), n(2), p.get(3)) {
                    (Some(x), Some(y), Some(r), Some(c)) => Some((x, y, r, c.to_string())),
                    _ => return Err(format!("--spotlight {s}: write x,y,radius,color (x a fraction of the width, y and radius of the height, e.g. 0.5,0.45,0.28,#e8b4b0)")),
                }
            }
        };
        let clip = Clip::parse(clip)?;
        let first = clip.sample(0.0)?;
        let sides: Vec<Side> = [("right", Side::Right), ("left", Side::Left)]
            .into_iter()
            .filter(|(name, _)| first.pose["hands"].get(*name).is_some())
            .map(|(_, side)| side)
            .collect();
        let models = std::thread::scope(|s| {
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
        let looks = looks(&clip, look_args)?
            .into_iter()
            .map(|look| {
                let tone = TONES.iter().find(|(_, rgb)| *rgb == look.skin).map_or_else(
                    || format!("#{:02x}{:02x}{:02x}", look.skin[0], look.skin[1], look.skin[2]),
                    |(name, _)| name.to_string(),
                );
                let label = format!("{} · {tone}", look.style.name());
                let mut styler = Styler::new(look, WIDTH as u32, HEIGHT as u32);
                let hands = models.iter().map(|m| styler.dress(m)).collect();
                Dressed { styler, hands, label }
            })
            .collect();
        let look_every = look_args.looks.is_some().then_some(look_args.look_every.max(0.1));
        Ok(Self { media, clip, looks, look_every, background: background.to_string(), spotlight })
    }

    pub fn duration_seconds(&self) -> f64 {
        self.clip.duration
    }

    /// The look at `t` seconds.
    fn dressed(&self, t: f64) -> &Dressed {
        let i = self.look_every.map_or(0, |every| (t / every) as usize % self.looks.len());
        &self.looks[i]
    }

    /// The hands at `t` seconds as an image the size of the video (transparent around them).
    fn hands_image(&self, t: f64) -> Option<fframes::media::ImageData<'static>> {
        let look = self.dressed(t);
        // Sub-frames for motion blur sample the clip within the shutter; a failed sample
        // (none expected: the clip parsed) leaves that sub-frame empty.
        let pixels = look.styler.render(|dt| {
            let mut scene = look.styler.stage();
            let at = (t + dt as f64 / FPS as f64).clamp(0.0, self.clip.duration);
            let Ok(frame) = self.clip.sample(at) else { return (scene, Camera::perspective(35.0)) };
            if let Ok(solved) = handkit_core::solve_value(&frame.pose) {
                for model in &look.hands {
                    model.add_to(&mut scene).pose(&mut scene, &solved);
                }
            }
            let cam = frame.pose.get("camera").cloned().unwrap_or(serde_json::json!({ "position": [0.0, 0.1, 0.6], "target": [0.0, 0.05, 0.0], "fov": 35 }));
            let v = |k: &str, i: usize| cam[k][i].as_f64().unwrap_or(0.0) as f32;
            let camera = Camera::perspective(cam["fov"].as_f64().unwrap_or(35.0) as f32)
                .at(v("position", 0), v("position", 1), v("position", 2))
                .look_at(vec3(v("target", 0), v("target", 1), v("target", 2)));
            (scene, camera)
        });
        Some(pixels.into_image())
    }
}

impl std::fmt::Debug for FframesClipVideo<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FframesClipVideo").field("duration", &self.clip.duration).finish()
    }
}

impl Video for FframesClipVideo<'_> {
    const FPS: usize = FPS;
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
        let t = frame.seconds() as f64;
        let hands = self.hands_image(t);
        let (w, h) = (WIDTH as f32, HEIGHT as f32);
        let dressed = self.dressed(t);
        let style = dressed.styler.look.style;
        // Cycling looks: each on its own backdrop, its name in the corner.
        let (background, label) = match self.look_every {
            Some(_) => (backdrop(style), Some(dressed.label.as_str())),
            None => (self.background.as_str(), None),
        };
        let ink = if matches!(style, Style::Arcane | Style::Realistic) { "#f4efe6" } else { "#1d1b1a" };
        fframes::svgr!(
            <svg xmlns="http://www.w3.org/2000/svg" viewBox={format!("0 0 {WIDTH} {HEIGHT}")} width={WIDTH} height={HEIGHT}>
                <defs>
                    <radialGradient id="spot" cx="0.5" cy="0.5" r="0.5">
                        <stop offset="0" stop-color={self.spotlight.as_ref().map(|s| s.3.as_str()).unwrap_or("#ffffff")} stop-opacity="1" />
                        <stop offset="0.72" stop-color={self.spotlight.as_ref().map(|s| s.3.as_str()).unwrap_or("#ffffff")} stop-opacity="0.85" />
                        <stop offset="1" stop-color={self.spotlight.as_ref().map(|s| s.3.as_str()).unwrap_or("#ffffff")} stop-opacity="0" />
                    </radialGradient>
                </defs>
                <rect width={WIDTH} height={HEIGHT} fill={background} />
                {match &self.spotlight {
                    Some((x, y, r, _)) => fframes::svgr!(<circle cx={x * w} cy={y * h} r={r * h} fill="url(#spot)" />),
                    None => Svgr::empty(),
                }}
                {match &hands {
                    Some(image) => fframes::svgr!(<image href={image.href()} x="0" y="0" width={WIDTH} height={HEIGHT} />),
                    None => Svgr::empty(),
                }}
                {match label {
                    Some(name) => fframes::svgr!(
                        <text x={h * 0.06} y={h * 0.93} font-family="DM Sans" font-size={h * 0.06} font-weight="500" fill={ink} letter-spacing="1">{name}</text>
                    ),
                    None => Svgr::empty(),
                }}
            </svg>
        )
    }
}
