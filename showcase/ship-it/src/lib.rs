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


// ---- The "Ship it" shot: one release card on paper, one physical button under the fingertip. ----
//
// The film shows this shot zoomed 1.55x-1.42x around (50%, 56%), so only about x 285..1635,
// y 180..940 is ever seen: everything sits inside that. The hand comes in from the bottom
// right and covers that corner, so the copy and the button label sit top and left of the
// fingertip, and the only follow-through (the status turning "Live") is top right.

/// The right index fingertip per frame from `tip_track.py`: [screen x px, screen y px,
/// distance from the camera in m].
const TIP: &str = include_str!("tip.json");
/// The screen's distance from the camera: the fingertip reaches it during the tap.
const SCREEN_D: f64 = 0.800;
/// When the fingertip first touches the button.
const TAP: f64 = 2.52;

const INK: &str = "#0B0B0D";
const PAPER: &str = "#F4F2EE";
const ACCENT: &str = "#FF5B2E";
const FONT: &str = "Inter Tight";

/// The card: x, y, width, height.
const CARD: (f64, f64, f64, f64) = (508.0, 250.0, 904.0, 534.0);
/// Inner margin of the card.
const PAD: f64 = 72.0;
/// The button's face at rest: x, y, width, height (the fingertip lands at about 970, 640).
const BTN: (f64, f64, f64, f64) = (580.0, 580.0, 760.0, 120.0);
/// How far the face stands above the card: the visible edge under it, pressed away by the tap.
const LIP: f64 = 12.0;

fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}
fn lin(t: f64, a: f64, b: f64) -> f64 {
    clamp01((t - a) / (b - a))
}
fn smooth(t: f64, a: f64, b: f64) -> f64 {
    let u = lin(t, a, b);
    u * u * (3.0 - 2.0 * u)
}
fn out_cubic(u: f64) -> f64 {
    1.0 - (1.0 - u).powi(3)
}
fn out_back(u: f64) -> f64 {
    let c = 1.6;
    1.0 + (c + 1.0) * (u - 1.0).powi(3) + c * (u - 1.0).powi(2)
}
fn hex(c: &str) -> [f64; 3] {
    let v = |i: usize| u8::from_str_radix(&c[1 + 2 * i..3 + 2 * i], 16).unwrap_or(0) as f64;
    [v(0), v(1), v(2)]
}
fn mix(a: &str, b: &str, t: f64) -> String {
    let (a, b) = (hex(a), hex(b));
    let c = |i: usize| (a[i] + (b[i] - a[i]) * clamp01(t)).round().clamp(0.0, 255.0) as u8;
    format!("#{:02x}{:02x}{:02x}", c(0), c(1), c(2))
}

impl FframesClipVideo<'_> {
    fn tip(&self, t: f64) -> [f64; 3] {
        let track: Vec<[f64; 3]> = serde_json::from_str(TIP).unwrap_or_default();
        if track.is_empty() {
            return [0.0, 0.0, SCREEN_D - 0.05];
        }
        let f = (t * FPS as f64).round().max(0.0) as usize;
        track[f.min(track.len() - 1)]
    }
}

impl Video for FframesClipVideo<'_> {
    const FPS: usize = FPS;
    const WIDTH: usize = WIDTH;
    const HEIGHT: usize = HEIGHT;
    const BACKGROUND_COLOR: Color = Color::WHITE;

    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(self.clip.duration as f32)
    }

    fn audio(&self) -> AudioMap<'_> {
        AudioMap::none()
    }

    fn render_frame<'a>(&'a self, mut frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let t = frame.seconds() as f64;
        let hands = self.hands_image(t);

        // The hand's shadow on the screen: the light is above and to the right, so it falls
        // down and to the left, out from under the hand where it shows on the white card. It
        // is softer the higher the fingertip is above the glass and closes in on the touch.
        let tip = self.tip(t);
        let gap = (SCREEN_D - tip[2]).max(0.0);
        let off = gap * 950.0;
        let (sdx, sdy, sblur) = (-off * 0.45, off * 0.9, 4.0 + gap * 160.0);
        let sop = 0.40 - (gap * 1.2).min(0.1);

        // The press follows the fingertip into the glass, then the button latches lower:
        // a shipped release does not pop back up all the way.
        let touch = smooth(tip[2], 0.784, 0.797);
        let latch_rise = out_back(lin(t, 2.70, 3.00));
        let dep = if t < TAP + 0.03 { touch * LIP } else { (touch * LIP).max(LIP - 6.0 * latch_rise) };
        // Ink floods the face out from under the fingertip: orange (do it) becomes ink (done)
        // without passing through a muddy mix of the two.
        let flood = out_cubic(lin(t, 2.56, 2.80));
        let flood_r = 1.0 + 420.0 * flood;
        let edge = mix("#C2401B", "#000000", smooth(t, 2.62, 2.70));
        let label_in = out_cubic(lin(t, 2.70, 2.98));
        let check = smooth(t, 2.76, 3.02);

        // Follow-through: the release goes live a beat after the button settles.
        let live = smooth(t, 2.98, 3.14);

        // The card settles in at the start.
        let intro = out_cubic(lin(t, 0.0, 0.8));
        let card_dy = (1.0 - intro) * 28.0;

        let (cx, cy, cw, ch) = CARD;
        let (bx, by, bw, bh) = BTN;
        let left = cx + PAD;
        let right = cx + cw - PAD;
        let fy = by + dep; // the face's top
        let r = 26.0;

        // Status, top right, right-aligned: a dot then the word.
        let status_font = |w: u16| fframes::FontQuery { family: FONT, size: 30, weight: w, ..Default::default() };
        let w_ready = frame.text_width(ctx, status_font(600), "Ready to ship").unwrap_or(180) as f64;
        let w_live = frame.text_width(ctx, status_font(600), "Live").unwrap_or(56) as f64;
        let dot_ready = right - w_ready - 22.0;
        let dot_live = right - w_live - 22.0;
        let dot_x = dot_ready + (dot_live - dot_ready) * live;
        let dot_r = 8.0 * (1.0 + 0.5 * (lin(t, 2.98, 3.30) * std::f64::consts::PI).sin());

        let check_path = format!("M{} {} L{} {} L{} {}", left + 36.0, fy + 61.0, left + 52.0, fy + 77.0, left + 82.0, fy + 45.0);
        let arrow_path = format!(
            "M{} {} L{} {} M{} {} L{} {} L{} {}",
            bx + bw - 92.0, fy + 60.0, bx + bw - 52.0, fy + 60.0,
            bx + bw - 70.0, fy + 42.0, bx + bw - 52.0, fy + 60.0, bx + bw - 70.0, fy + 78.0
        );

        fframes::svgr!(
            <svg xmlns="http://www.w3.org/2000/svg" viewBox={format!("0 0 {WIDTH} {HEIGHT}")} width={WIDTH} height={HEIGHT}>
                <defs>
                    <filter id="cardshadow" x="-20%" y="-20%" width="140%" height="160%">
                        <feGaussianBlur stdDeviation="34" />
                    </filter>
                    <filter id="btnshadow" x="-10%" y="-60%" width="120%" height="260%">
                        <feGaussianBlur stdDeviation="9" />
                    </filter>
                    <filter id="handshadow" x="-10%" y="-10%" width="120%" height="120%">
                        <feColorMatrix type="matrix" values={format!("0 0 0 0 0.043  0 0 0 0 0.043  0 0 0 0 0.05  0 0 0 {sop:.3} 0")} />
                        <feGaussianBlur stdDeviation={sblur} />
                    </filter>
                    <clipPath id="facetext">
                        <rect x={bx} y={fy} width={bw} height={bh} rx={r} />
                    </clipPath>
                </defs>
                <rect width={WIDTH} height={HEIGHT} fill={PAPER} />

                <g opacity={intro} transform={format!("translate(0 {card_dy:.2})")}>
                    <rect x={cx + 30.0} y={cy + 40.0} width={cw - 60.0} height={ch - 20.0} rx="36" fill={INK} opacity="0.08" filter="url(#cardshadow)" />
                    <rect x={cx} y={cy} width={cw} height={ch} rx="32" fill="#FFFFFF" stroke={INK} stroke-opacity="0.07" stroke-width="1.5" />
                    <rect x={left} y={cy + 79.0} width="14" height="14" rx="2.5" fill={ACCENT} />
                    <text x={left + 28.0} y={cy + 97.0} font-family={FONT} font-weight="600" font-size="30" fill={INK} fill-opacity="0.55">{"Release"}</text>
                    <circle cx={dot_x} cy={cy + 86.0} r={dot_r} fill={mix("#C9C6C0", ACCENT, live)} />
                    <text x={right} y={cy + 97.0} text-anchor="end" font-family={FONT} font-weight="600" font-size="30" fill={INK} fill-opacity={0.5 * (1.0 - live)}>{"Ready to ship"}</text>
                    <text x={right} y={cy + 97.0} text-anchor="end" font-family={FONT} font-weight="600" font-size="30" fill={INK} opacity={live}>{"Live"}</text>
                    <text x={left - 5.0} y={cy + 221.0} font-family={FONT} font-weight="800" font-size="120" letter-spacing="-5" fill={INK}>{"Handkit 0.1"}</text>
                    <text x={left} y={cy + 273.0} font-family={FONT} font-weight="500" font-size="36" letter-spacing="-0.4" fill={INK} fill-opacity="0.58">{"Accurate 3D hands for code-made video."}</text>
                    <rect x={bx + 14.0} y={by + 26.0 + dep * 0.3} width={bw - 28.0} height={bh} rx={r} fill={INK} opacity={0.16 * (1.0 - 0.6 * dep / LIP)} filter="url(#btnshadow)" />
                    <rect x={bx} y={by + LIP} width={bw} height={bh} rx={r} fill={edge.clone()} />
                    <rect x={bx} y={fy} width={bw} height={bh} rx={r} fill={if flood < 1.0 { ACCENT } else { INK }} />
                    <g clip-path="url(#facetext)">
                        <g opacity={if flood < 1.0 { 1.0 } else { 0.0 }}>
                            <text x={left + 40.0} y={fy + 77.0} font-family={FONT} font-weight="800" font-size="50" letter-spacing="-1.4" fill={INK}>{"Ship it"}</text>
                            <path d={arrow_path} fill="none" stroke={INK} stroke-width="6" stroke-linecap="round" stroke-linejoin="round" />
                        </g>
                        <circle cx="970" cy={fy + 58.0} r={flood_r} fill={INK} opacity={if t >= 2.56 { 1.0 } else { 0.0 }} />
                        <g opacity={label_in} transform={format!("translate(0 {:.2})", 26.0 * (1.0 - label_in))}>
                            <path d={check_path} fill="none" stroke={ACCENT} stroke-width="7" stroke-linecap="round" stroke-linejoin="round" stroke-dasharray="70" stroke-dashoffset={70.0 * (1.0 - check)} />
                            <text x={left + 104.0} y={fy + 77.0} font-family={FONT} font-weight="800" font-size="50" letter-spacing="-1.4" fill={PAPER}>{"Shipped"}</text>
                        </g>
                    </g>
                </g>

                {match &hands {
                    Some(image) => fframes::svgr!(<g>
                        <image href={image.href()} x={sdx} y={sdy} width={WIDTH} height={HEIGHT} filter="url(#handshadow)" />
                        <image href={image.href()} x="0" y="0" width={WIDTH} height={HEIGHT} />
                    </g>),
                    None => Svgr::empty(),
                }}
            </svg>
        )
    }
}
