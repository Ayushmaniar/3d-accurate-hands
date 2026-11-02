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

// ---- The "Ship it" shot: a deploy card under the hand, the button reacting to the tap. ----

/// The right index fingertip per frame from `tip_track.py`: [screen x px, screen y px,
/// distance from the camera in m].
const TIP: &str = include_str!("tip.json");
/// The screen's distance from the camera: the fingertip reaches it during the tap.
const SCREEN_D: f64 = 0.800;
/// When the fingertip first touches the button.
const TAP: f64 = 2.52;
/// The button: centre x, y, width, height.
const BTN: (f64, f64, f64, f64) = (960.0, 640.0, 360.0, 108.0);
/// Where the fingertip lands.
const TOUCH: (f64, f64) = (970.0, 640.0);

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
    let c = 1.9;
    1.0 + (c + 1.0) * (u - 1.0).powi(3) + c * (u - 1.0).powi(2)
}
fn mix(a: [f64; 3], b: [f64; 3], t: f64) -> String {
    let c = |i: usize| (a[i] + (b[i] - a[i]) * t).round().clamp(0.0, 255.0) as u8;
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

fn check_icon<'a>(cx: f64, cy: f64, s: f64, color: &'static str) -> Svgr<'a> {
    fframes::svgr!(<g>
        <circle cx={cx} cy={cy} r={11.0 * s} fill={color} opacity="0.18" />
        <path d={format!("M{} {} L{} {} L{} {}", cx - 5.0 * s, cy, cx - 1.5 * s, cy + 4.0 * s, cx + 5.5 * s, cy - 4.5 * s)} fill="none" stroke={color} stroke-width={2.6 * s} stroke-linecap="round" stroke-linejoin="round" />
    </g>)
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
        let font = "DM Sans";

        // The fingertip's height above the screen drives the hand's shadow on it.
        let tip = self.tip(t);
        let gap = (SCREEN_D - tip[2]).max(0.0);
        let off = gap * 800.0;
        let (sdx, sdy, sblur) = (off * 0.55, off, 2.5 + gap * 150.0);
        let sop = 0.7 - (gap * 3.0).min(0.3);

        // Button states.
        let intro = out_cubic(lin(t, 0.0, 0.7));
        let hover = smooth(t, 1.25, 1.7) * (1.0 - smooth(t, 2.6, 2.75));
        let press_in = smooth(t, TAP - 0.04, TAP + 0.04);
        let release = out_back(lin(t, 2.70, 3.05));
        let press = press_in * (1.0 - release);
        let ok = smooth(t, 2.62, 2.92);
        let scale = 1.0 + 0.025 * hover - 0.06 * press;
        let (bx, by, bw, bh) = BTN;
        let (tx, ty) = TOUCH;
        let idle_fill = mix([99.0, 102.0, 241.0], [124.0, 127.0, 252.0], hover);
        let fill = if ok > 0.0 { mix([124.0, 127.0, 252.0], [22.0, 163.0, 74.0], ok) } else { idle_fill };
        let fill_dark = mix([67.0, 56.0, 202.0], [21.0, 128.0, 61.0], ok);
        let ripple_u = lin(t, TAP, TAP + 0.65);
        let ripple_r = 1.0 + out_cubic(ripple_u) * 330.0;
        let ripple_op = if t >= TAP { 0.55 * (1.0 - ripple_u) } else { 0.0 };
        let ring_u = lin(t, TAP + 0.05, TAP + 0.85);
        let ring_op = if t >= TAP + 0.05 { 0.7 * (1.0 - ring_u) } else { 0.0 };
        let ring_pad = out_cubic(ring_u) * 46.0;
        let glow = ok * (0.55 + 0.15 * (1.0 - lin(t, 3.0, 4.0)));
        let check_draw = smooth(t, 2.78, 3.05);

        // Deploy progress after the tap, then live.
        let prog = out_cubic(lin(t, 2.75, 3.95));
        let live = smooth(t, 3.95, 4.15);
        let chip_pop = out_back(lin(t, 3.95, 4.3));
        let chip_scale = 1.0 + 0.15 * (lin(t, 3.95, 4.05) - lin(t, 4.05, 4.3)).max(0.0) + 0.0 * chip_pop;
        let toast = out_back(lin(t, 4.25, 4.75));

        let lbl_a = 1.0 - smooth(t, 2.62, 2.8);
        let lbl_b = smooth(t, 2.74, 2.95);
        let burst = lin(t, TAP + 0.02, TAP + 0.6);
        let card_y = (1.0 - intro) * 30.0;

        let sparks: Svgr = (0..14)
            .map(|i| {
                let a = i as f64 / 14.0 * std::f64::consts::TAU + 0.2;
                let u = out_cubic(burst);
                let (ex, ey) = (a.cos(), a.sin());
                let (rx0, ry0) = (195.0 + 110.0 * u, 66.0 + 50.0 * u);
                let len = 0.12 * (1.0 - burst) + 0.02;
                let op = if t > TAP + 0.02 { (1.0 - burst) * 0.9 } else { 0.0 };
                fframes::svgr!(<line x1={bx + ex * rx0} y1={by + ey * ry0} x2={bx + ex * rx0 * (1.0 + len)} y2={by + ey * ry0 * (1.0 + len)}
                    stroke="#86efac" stroke-width="5" stroke-linecap="round" opacity={op} />)
            })
            .collect();

        let pills: Svgr = [("Tests 412/412", 690.0), ("Build 1m 42s", 960.0), ("Preview approved", 1230.0)]
            .into_iter()
            .map(|(label, cx): (&'static str, f64)| {
                fframes::svgr!(<g>
                    <rect x={cx - 120.0} y="470" width="240" height="52" rx="26" fill="#1c2620" stroke="#24452f" stroke-width="2" />
                    {check_icon(cx - 88.0, 496.0, 1.0, "#4ade80")}
                    <text x={cx + 14.0} y="503" text-anchor="middle" font-family={font} font-weight="500" font-size="21" fill="#b7e4c7">{label}</text>
                </g>)
            })
            .collect();

        let status = if live > 0.5 { "Live" } else { "Ready" };
        let deploying = format!("Deploying  ·  {} of 3 regions", ((prog * 3.0).floor() as i32).clamp(0, 3));

        fframes::svgr!(
            <svg xmlns="http://www.w3.org/2000/svg" viewBox={format!("0 0 {WIDTH} {HEIGHT}")} width={WIDTH} height={HEIGHT}>
                <defs>
                    <radialGradient id="bg" cx="0.5" cy="0.42" r="0.75">
                        <stop offset="0" stop-color="#1c2030" />
                        <stop offset="1" stop-color="#0a0b10" />
                    </radialGradient>
                    <linearGradient id="card" x1="0" y1="0" x2="0" y2="1">
                        <stop offset="0" stop-color="#1a1d27" />
                        <stop offset="1" stop-color="#14161e" />
                    </linearGradient>
                    <linearGradient id="btn" x1="0" y1="0" x2="0" y2="1">
                        <stop offset="0" stop-color={fill.clone()} />
                        <stop offset="1" stop-color={fill_dark.clone()} />
                    </linearGradient>
                    <filter id="glow" x="-50%" y="-80%" width="200%" height="260%">
                        <feGaussianBlur stdDeviation="28" />
                    </filter>
                    <filter id="cardshadow" x="-20%" y="-20%" width="140%" height="150%">
                        <feGaussianBlur stdDeviation="40" />
                    </filter>
                    <filter id="handshadow" x="-10%" y="-10%" width="120%" height="120%">
                        <feColorMatrix type="matrix" values={format!("0 0 0 0 0.02  0 0 0 0 0.02  0 0 0 0 0.05  0 0 0 {sop:.3} 0")} />
                        <feGaussianBlur stdDeviation={sblur} />
                    </filter>
                    <filter id="toastshadow" x="-20%" y="-40%" width="140%" height="200%">
                        <feGaussianBlur stdDeviation="18" />
                    </filter>
                    <clipPath id="btnclip">
                        <rect x={bx - bw / 2.0} y={by - bh / 2.0} width={bw} height={bh} rx={bh / 2.0} />
                    </clipPath>
                </defs>
                <rect width={WIDTH} height={HEIGHT} fill="url(#bg)" />

                <g opacity={intro} transform={format!("translate(0 {card_y:.2})")}>
                    <rect x="440" y="200" width="1040" height="700" rx="28" fill="#000000" opacity="0.55" filter="url(#cardshadow)" />
                    <rect x="420" y="170" width="1080" height="740" rx="26" fill="url(#card)" stroke="#2a2f3d" stroke-width="2" />
                    <path d="M420 238 L1500 238" stroke="#232733" stroke-width="2" />
                    <circle cx="462" cy="204" r="8" fill="#ef5f57" />
                    <circle cx="490" cy="204" r="8" fill="#f5bd2f" />
                    <circle cx="518" cy="204" r="8" fill="#29c840" />
                    <text x="960" y="212" text-anchor="middle" font-family={font} font-weight="500" font-size="22" fill="#7d8496">{"Deploys"}</text>
                    <g transform={format!("translate(1404 204) scale({chip_scale:.3}) translate(-1404 -204)")}>
                        <rect x="1346" y="188" width="116" height="34" rx="17" fill={mix([58.0, 46.0, 20.0], [20.0, 58.0, 36.0], live)} />
                        <circle cx="1370" cy="205" r="6" fill={mix([245.0, 189.0, 47.0], [74.0, 222.0, 128.0], live)} />
                        <text x="1414" y="213" text-anchor="middle" font-family={font} font-weight="500" font-size="20" fill={mix([245.0, 208.0, 120.0], [134.0, 239.0, 172.0], live)}>{status}</text>
                    </g>

                    <text x="960" y="318" text-anchor="middle" font-family={font} font-weight="500" font-size="20" letter-spacing="4" fill="#8b93a7">{"PRODUCTION RELEASE"}</text>
                    <text x="960" y="390" text-anchor="middle" font-family={font} font-weight="500" font-size="58" fill="#f3f4f8">{"v2.4.0  Checkout redesign"}</text>
                    <text x="960" y="438" text-anchor="middle" font-family={font} font-weight="500" font-size="24" fill="#7d8496">{"main  ·  12 commits  ·  3 reviewers"}</text>
                    {pills}

                    <rect x={bx - bw / 2.0 + 10.0} y={by - bh / 2.0 + 18.0} width={bw - 20.0} height={bh} rx={bh / 2.0} fill="#22c55e" opacity={glow} filter="url(#glow)" />
                    <rect x={bx - bw / 2.0 + 14.0} y={by - bh / 2.0 + 22.0} width={bw - 28.0} height={bh - 10.0} rx={bh / 2.0} fill="#6366f1" opacity={0.45 * (1.0 - ok) * (0.6 + 0.4 * hover)} filter="url(#glow)" />
                    <rect x={bx - bw / 2.0 - ring_pad * 1.6} y={by - bh / 2.0 - ring_pad} width={bw + ring_pad * 3.2} height={bh + ring_pad * 2.0} rx={bh / 2.0 + ring_pad} fill="none" stroke="#4ade80" stroke-width="3" opacity={ring_op} />
                    <g transform={format!("translate({bx} {by}) scale({scale:.4}) translate({} {})", -bx, -by)}>
                        <rect x={bx - bw / 2.0} y={by - bh / 2.0 + 6.0} width={bw} height={bh} rx={bh / 2.0} fill={fill_dark.clone()} opacity={0.9 * (1.0 - press)} />
                        <rect x={bx - bw / 2.0} y={by - bh / 2.0 + 4.0 * press} width={bw} height={bh} rx={bh / 2.0} fill="url(#btn)" />
                        <rect x={bx - bw / 2.0 + 3.0} y={by - bh / 2.0 + 3.0 + 4.0 * press} width={bw - 6.0} height={bh / 2.0} rx={bh / 2.0 - 3.0} fill="#ffffff" opacity={0.10 + 0.05 * hover} />
                        <g clip-path="url(#btnclip)">
                            <circle cx={tx} cy={ty} r={ripple_r} fill="#ffffff" opacity={ripple_op} />
                        </g>
                        <g opacity={lbl_a} transform={format!("translate(0 {:.2})", -14.0 * (1.0 - lbl_a) + 4.0 * press)}>
                            <text x={bx - 18.0} y={by + 15.0} text-anchor="middle" font-family={font} font-weight="500" font-size="42" fill="#ffffff">{"Ship it"}</text>
                            <path d={format!("M{} {} L{} {} M{} {} L{} {} L{} {}", bx + 66.0, by + 1.0, bx + 98.0, by + 1.0, bx + 86.0, by - 11.0, bx + 98.0, by + 1.0, bx + 86.0, by + 13.0)} fill="none" stroke="#ffffff" stroke-width="5" stroke-linecap="round" stroke-linejoin="round" />
                        </g>
                        <g opacity={lbl_b} transform={format!("translate(0 {:.2})", 14.0 * (1.0 - lbl_b))}>
                            <path d={format!("M{} {} L{} {} L{} {}", bx - 112.0, by + 1.0, bx - 98.0, by + 15.0, bx - 72.0, by - 13.0)} fill="none" stroke="#ffffff" stroke-width="7" stroke-linecap="round" stroke-linejoin="round" stroke-dasharray="70" stroke-dashoffset={70.0 * (1.0 - check_draw)} />
                            <text x={bx + 24.0} y={by + 15.0} text-anchor="middle" font-family={font} font-weight="500" font-size="42" fill="#ffffff">{"Shipped"}</text>
                        </g>
                    </g>
                    {sparks}

                    <text x="960" y="762" text-anchor="middle" font-family={font} font-weight="500" font-size="22" fill="#6b7285" opacity={1.0 - smooth(t, 2.6, 2.8)}>{"Deploys main to production in 3 regions"}</text>
                    <g opacity={smooth(t, 2.7, 2.9)}>
                        <rect x="760" y="774" width="400" height="8" rx="4" fill="#232836" />
                        <rect x="760" y="774" width={0.1 + 400.0 * prog} height="8" rx="4" fill={mix([99.0, 102.0, 241.0], [34.0, 197.0, 94.0], live.max(prog * 0.6))} />
                        <text x="960" y="752" text-anchor="middle" font-family={font} font-weight="500" font-size="22" fill="#9aa1b2" opacity={1.0 - live}>{deploying}</text>
                        <text x="960" y="752" text-anchor="middle" font-family={font} font-weight="500" font-size="22" fill="#86efac" opacity={live}>{"Live in production  ·  38s"}</text>
                    </g>
                </g>

                {match &hands {
                    Some(image) => fframes::svgr!(<g>
                        <image href={image.href()} x={sdx} y={sdy} width={WIDTH} height={HEIGHT} filter="url(#handshadow)" />
                        <image href={image.href()} x="0" y="0" width={WIDTH} height={HEIGHT} />
                    </g>),
                    None => Svgr::empty(),
                }}

                <g opacity={lin(t, 4.25, 4.45)} transform={format!("translate({:.2} 0)", (1.0 - toast) * 80.0)}>
                    <rect x="1490" y="64" width="380" height="88" rx="18" fill="#000000" opacity="0.5" filter="url(#toastshadow)" />
                    <rect x="1480" y="52" width="380" height="88" rx="18" fill="#1b2a22" stroke="#2c5a3d" stroke-width="2" />
                    {check_icon(1530.0, 96.0, 1.6, "#4ade80")}
                    <text x="1566" y="90" font-family={font} font-weight="500" font-size="24" fill="#e8f7ee">{"v2.4.0 is live"}</text>
                    <text x="1566" y="120" font-family={font} font-weight="500" font-size="19" fill="#86b79a">{"Shipped by you  ·  just now"}</text>
                </g>
            </svg>
        )
    }
}
