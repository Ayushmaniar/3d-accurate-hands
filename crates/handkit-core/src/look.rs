//! How a hand is drawn: a pose's or clip's top-level `look`. Core only checks it and passes it
//! on; renderers (handkit-fframes' `look` module, the three.js preview) draw it.
//!
//! ```json
//! "look": { "style": "toon", "skin": "tan", "nail": "#e9c4b8", "outline": 3, "motion_blur": 4, "arm": "fade" }
//! ```

use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Style {
    /// Lit skin: soft key and rim light, screen-space subsurface scattering, warm shadows.
    #[default]
    Realistic,
    /// Three-band cel shading and an ink outline.
    Toon,
    /// Toon with Ben-Day halftone dots in the shadows, colour misregistration and a thick,
    /// uneven ink line.
    Spiderverse,
    /// Painterly: soft cel bands with warm/cool shifts, brush strokes, fine dark line work.
    Arcane,
    /// Matte modelling clay: rough, lumpy, with fingerprints.
    Clay,
}

impl Style {
    pub const ALL: [Style; 5] = [Style::Realistic, Style::Toon, Style::Spiderverse, Style::Arcane, Style::Clay];

    pub fn name(self) -> &'static str {
        match self {
            Style::Realistic => "realistic",
            Style::Toon => "toon",
            Style::Spiderverse => "spiderverse",
            Style::Arcane => "arcane",
            Style::Clay => "clay",
        }
    }

    pub fn parse(s: &str) -> Result<Style, String> {
        Style::ALL
            .into_iter()
            .find(|x| x.name() == s.trim().to_ascii_lowercase())
            .ok_or_else(|| format!("look style \"{s}\": use realistic, toon, spiderverse, arcane or clay"))
    }

    /// The default ink line width in pixels at 1080p.
    pub fn default_outline(self) -> f64 {
        match self {
            Style::Realistic | Style::Clay => 0.0,
            Style::Toon => 3.0,
            Style::Spiderverse => 4.5,
            Style::Arcane => 1.6,
        }
    }
}

/// The named skin tones (sRGB albedo).
pub const TONES: [(&str, [u8; 3]); 5] = [
    ("light", [0xf0, 0xc8, 0xb2]),
    ("medium", [0xd8, 0xa6, 0x8a]),
    ("tan", [0xbc, 0x86, 0x66]),
    ("brown", [0x8e, 0x5e, 0x46]),
    ("dark", [0x5e, 0x3e, 0x30]),
];

/// A colour: a tone name (`light`, `medium`, `tan`, `brown`, `dark`) or `#rrggbb`.
pub fn parse_color(s: &str) -> Result<[u8; 3], String> {
    let t = s.trim().to_ascii_lowercase();
    if let Some((_, c)) = TONES.iter().find(|(n, _)| *n == t) {
        return Ok(*c);
    }
    let hex = t.strip_prefix('#').unwrap_or("");
    let ok = hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit());
    if !ok {
        return Err(format!("colour \"{s}\": write light, medium, tan, brown, dark or #rrggbb"));
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).unwrap();
    Ok([byte(0), byte(2), byte(4)])
}

/// Where the forearm ends: the rig stops at the elbow.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum Arm {
    /// Skin up to the elbow, where the forearm ends in a rounded cap.
    #[default]
    Bare,
    /// The forearm fades to transparent over its last 30% toward the elbow.
    Fade,
    /// A rolled-up cloth sleeve over the forearm from the elbow: its colour (sRGB) and how far
    /// it reaches toward the wrist (meters).
    Sleeve { color: [u8; 3], length: f64 },
}

impl Arm {
    /// A sleeve's default length (meters from the elbow).
    pub const SLEEVE_LENGTH: f64 = 0.08;

    /// `"bare"`, `"fade"` or `{"sleeve": "#rrggbb", "length": 0.08}` (length optional).
    pub fn from_value(v: &Value) -> Result<Arm, String> {
        const HELP: &str = "look arm: \"bare\", \"fade\" or {\"sleeve\": \"#rrggbb\", \"length\": 0.08}";
        match v {
            Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
                "bare" => Ok(Arm::Bare),
                "fade" => Ok(Arm::Fade),
                _ => Err(HELP.into()),
            },
            Value::Object(o) => {
                if let Some(k) = o.keys().find(|k| !["sleeve", "length"].contains(&k.as_str())) {
                    return Err(format!("{HELP} (unknown key \"{k}\")"));
                }
                let color = o.get("sleeve").and_then(Value::as_str).ok_or(HELP)?;
                let color = parse_color(color).map_err(|e| format!("look arm sleeve: {e}"))?;
                let length = match o.get("length") {
                    None | Some(Value::Null) => Arm::SLEEVE_LENGTH,
                    Some(x) => x.as_f64().filter(|x| (0.02..=0.2).contains(x)).ok_or("look arm length: meters from the elbow, 0.02 to 0.2")?,
                };
                Ok(Arm::Sleeve { color, length })
            }
            _ => Err(HELP.into()),
        }
    }

    /// As JSON, the sleeve's colour in sRGB bytes: `"bare"`, `"fade"` or
    /// `{"sleeve": [r, g, b], "length": ..}`.
    pub fn to_value(&self) -> Value {
        match self {
            Arm::Bare => json!("bare"),
            Arm::Fade => json!("fade"),
            Arm::Sleeve { color, length } => json!({ "sleeve": color, "length": length }),
        }
    }

    /// Whether the elbow may be in view: the forearm's end fades or is in a sleeve.
    pub fn hides_end(&self) -> bool {
        *self != Arm::Bare
    }
}

/// A wall behind the hands and a lamp in front of them that throws their shadows on it, as in
/// a shadow-puppet show.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wall {
    /// Its colour (sRGB).
    pub color: [u8; 3],
    /// How far behind the point the camera looks at it stands (meters, along the view), facing
    /// the camera and moving with it.
    pub behind: f64,
    /// A wall fixed in the world instead: a point on it (meters) and the way it faces.
    pub at: Option<[f64; 3]>,
    pub facing: [f64; 3],
    /// Where the lamp stands (world, meters); none puts it beside the camera, a little up and
    /// to the left, so the shadows fall down and to the right of the hands.
    pub lamp: Option<[f64; 3]>,
    /// The blur of the shadows' edges, 0 (a bare bulb) to 1 (a large soft lamp).
    pub softness: f64,
}

impl Wall {
    pub const COLOR: [u8; 3] = [0xe9, 0xdf, 0xcf];
    pub const BEHIND: f64 = 0.25;
    pub const SOFTNESS: f64 = 0.3;

    /// `true`, a colour, or `{"color": .., "behind": 0.25, "lamp": [x, y, z], "softness": 0.3}`
    /// (`"at": [x, y, z], "facing": [x, y, z]` in place of `behind` fix the wall in the world).
    pub fn from_value(v: &Value) -> Result<Wall, String> {
        const HELP: &str = "look wall: true, a colour, or {\"color\": \"#e9dfcf\", \"behind\": 0.25 or \"at\": [x, y, z] and \"facing\": [x, y, z], \"lamp\": [x, y, z], \"softness\": 0.3}";
        let mut wall = Wall { color: Wall::COLOR, behind: Wall::BEHIND, at: None, facing: [0.0, 0.0, 1.0], lamp: None, softness: Wall::SOFTNESS };
        match v {
            Value::Bool(true) => {}
            Value::String(s) if ["true", "on"].contains(&s.trim()) => {}
            Value::String(s) => wall.color = parse_color(s).map_err(|e| format!("look wall: {e}"))?,
            Value::Object(o) => {
                if let Some(k) = o.keys().find(|k| !["color", "behind", "at", "facing", "lamp", "softness"].contains(&k.as_str())) {
                    return Err(format!("{HELP} (unknown key \"{k}\")"));
                }
                if let Some(c) = o.get("color").filter(|c| !c.is_null()) {
                    wall.color = parse_color(c.as_str().ok_or(HELP)?).map_err(|e| format!("look wall color: {e}"))?;
                }
                if let Some(x) = o.get("behind").filter(|x| !x.is_null()) {
                    wall.behind = x.as_f64().filter(|x| (0.02..=5.0).contains(x)).ok_or("look wall behind: meters behind what the camera looks at, 0.02 to 5")?;
                }
                if let Some(x) = o.get("softness").filter(|x| !x.is_null()) {
                    wall.softness = x.as_f64().filter(|x| (0.0..=1.0).contains(x)).ok_or("look wall softness: 0 to 1")?;
                }
                let vec3 = |k: &str, what: &str| -> Result<Option<[f64; 3]>, String> {
                    let Some(v) = o.get(k).filter(|v| !v.is_null()) else { return Ok(None) };
                    let n: Vec<f64> = v.as_array().map(|a| a.iter().filter_map(Value::as_f64).collect()).unwrap_or_default();
                    match n[..] {
                        [x, y, z] if k != "facing" || x * x + y * y + z * z > 1e-12 => Ok(Some([x, y, z])),
                        _ => Err(format!("look wall {k}: {what} [x, y, z]")),
                    }
                };
                wall.lamp = vec3("lamp", "a position in meters")?;
                wall.at = vec3("at", "a point on the wall in meters")?;
                if let Some(f) = vec3("facing", "the direction the wall faces")? {
                    wall.facing = f;
                }
            }
            _ => return Err(HELP.into()),
        }
        Ok(wall)
    }
}

/// A checked look.Fields left `None` take the style's default (the renderer's choice).
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    pub style: Style,
    /// Skin albedo, sRGB. Default `medium`.
    pub skin: [u8; 3],
    /// Nail colour, sRGB; default: the skin, lighter and pinker.
    pub nail: Option<[u8; 3]>,
    /// Ink line width in pixels at 1080p (scaled with the frame height); 0 for none.
    pub outline: f64,
    /// Sub-frame renders averaged per frame; 1 is no motion blur.
    pub motion_blur: u32,
    /// The shutter angle in degrees (180 is half the frame time).
    pub shutter: f64,
    /// Halftone dot spacing in pixels at 1080p (spiderverse; 0 for none).
    pub halftone: Option<f64>,
    /// Colour misregistration offset in pixels at 1080p (spiderverse; 0 for none).
    pub misregister: Option<f64>,
    /// Strength of the surface texture: brush strokes (arcane), fingerprints and lumps (clay),
    /// 0 to 1.
    pub texture: Option<f64>,
    /// The subsurface scattering mix (realistic), 0 to 1.
    pub subsurface: Option<f64>,
    /// How the forearm ends at the elbow.
    pub arm: Arm,
    /// A lamp-lit wall behind the hands that shows their shadows.
    pub wall: Option<Wall>,
}

impl Default for Look {
    fn default() -> Self {
        Look::of(Style::Realistic)
    }
}

pub const KEYS: [&str; 12] = ["style", "skin", "nail", "outline", "motion_blur", "shutter", "halftone", "misregister", "texture", "subsurface", "arm", "wall"];

impl Look {
    /// The style's defaults.
    pub fn of(style: Style) -> Look {
        Look {
            style,
            skin: TONES[1].1,
            nail: None,
            outline: style.default_outline(),
            motion_blur: 1,
            shutter: 180.0,
            halftone: None,
            misregister: None,
            texture: None,
            subsurface: None,
            arm: Arm::Bare,
            wall: None,
        }
    }

    /// Reads a `look` object (or a bare style name).
    pub fn from_value(v: &Value) -> Result<Look, String> {
        if let Some(s) = v.as_str() {
            return Ok(Look::of(Style::parse(s)?));
        }
        let obj = v.as_object().ok_or("\"look\" is an object: {\"style\": .., \"skin\": .., ...}")?;
        for k in obj.keys() {
            if !KEYS.contains(&k.as_str()) {
                return Err(format!("unknown look key \"{k}\" (keys: {})", KEYS.join(", ")));
            }
        }
        let style = match obj.get("style") {
            None => Style::Realistic,
            Some(s) => Style::parse(s.as_str().ok_or("look style is a string")?)?,
        };
        let mut look = Look::of(style);
        let color = |k: &str| -> Result<Option<[u8; 3]>, String> {
            match obj.get(k) {
                None | Some(Value::Null) => Ok(None),
                Some(Value::String(s)) => parse_color(s).map(Some).map_err(|e| format!("look {k}: {e}")),
                Some(_) => Err(format!("look {k} is a string: a tone name or #rrggbb")),
            }
        };
        let num = |k: &str, lo: f64, hi: f64| -> Result<Option<f64>, String> {
            match obj.get(k) {
                None | Some(Value::Null) => Ok(None),
                Some(x) => {
                    let x = x.as_f64().ok_or(format!("look {k} must be a number"))?;
                    if !(lo..=hi).contains(&x) {
                        return Err(format!("look {k} {x}: from {lo} to {hi}"));
                    }
                    Ok(Some(x))
                }
            }
        };
        if let Some(c) = color("skin")? {
            look.skin = c;
        }
        look.nail = color("nail")?;
        if let Some(x) = num("outline", 0.0, 40.0)? {
            look.outline = x;
        }
        if let Some(x) = num("motion_blur", 1.0, 64.0)? {
            if x.fract() != 0.0 {
                return Err("look motion_blur is a whole number of sub-frames".into());
            }
            look.motion_blur = x as u32;
        }
        if let Some(x) = num("shutter", 0.0, 360.0)? {
            look.shutter = x;
        }
        look.halftone = num("halftone", 0.0, 100.0)?;
        look.misregister = num("misregister", 0.0, 40.0)?;
        look.texture = num("texture", 0.0, 1.0)?;
        look.subsurface = num("subsurface", 0.0, 1.0)?;
        if let Some(a) = obj.get("arm").filter(|a| !a.is_null()) {
            look.arm = Arm::from_value(a)?;
        }
        if let Some(w) = obj.get("wall").filter(|w| !w.is_null() && **w != Value::Bool(false)) {
            look.wall = Some(Wall::from_value(w)?);
        }
        Ok(look)
    }

    /// Reads the player's `--look style[,key=value...]`, e.g. `toon,skin=tan,outline=4`.
    pub fn parse_flag(s: &str) -> Result<Value, String> {
        let mut out = serde_json::Map::new();
        for (i, part) in s.split(',').map(str::trim).filter(|p| !p.is_empty()).enumerate() {
            match part.split_once('=') {
                None if i == 0 => {
                    out.insert("style".into(), json!(part));
                }
                None => return Err(format!("--look {s}: \"{part}\" needs key=value (after the style)")),
                Some((k, v)) => {
                    let v = v.trim();
                    let value = match v.parse::<f64>() {
                        Ok(x) => json!(x),
                        Err(_) => json!(v),
                    };
                    out.insert(k.trim().to_string(), value);
                }
            }
        }
        let v = Value::Object(out);
        Look::from_value(&v)?;
        Ok(v)
    }

    /// `over`'s keys over `base`'s (a flag over the clip's look). A different style resets
    /// nothing else: keys not given keep the style's defaults.
    pub fn merge(base: Option<&Value>, over: Option<&Value>) -> Option<Value> {
        let as_obj = |v: &Value| match v {
            Value::String(s) => json!({ "style": s }),
            v => v.clone(),
        };
        match (base, over) {
            (None, None) => None,
            (Some(b), None) => Some(as_obj(b)),
            (None, Some(o)) => Some(as_obj(o)),
            (Some(b), Some(o)) => {
                let mut b = as_obj(b);
                if let (Some(bo), Value::Object(oo)) = (b.as_object_mut(), as_obj(o)) {
                    bo.extend(oo);
                }
                Some(b)
            }
        }
    }

    /// This look in a person's colours: the skin, nail and arm of their own look
    /// (`people.<person>.look`) over this one's; the style and everything else stay this look's.
    /// A person who gives a skin and no nail gets the nail of their skin.
    pub fn for_person(&self, people: Option<&Value>, person: &str) -> Look {
        let mut l = self.clone();
        let Some(v) = people.and_then(|p| p.get(person)).and_then(|p| p.get("look")) else { return l };
        let Ok(own) = Look::from_value(v) else { return l };
        if v.get("skin").is_some() {
            (l.skin, l.nail) = (own.skin, own.nail);
        }
        if v.get("nail").is_some() {
            l.nail = own.nail;
        }
        if v.get("arm").is_some() {
            l.arm = own.arm;
        }
        l
    }

    /// The nail colour: given, or the skin lighter and pinker.
    pub fn nail_color(&self) -> [u8; 3] {
        self.nail.unwrap_or_else(|| {
            let [r, g, b] = self.skin.map(f64::from);
            let mix = |c: f64, t: f64| (c + (t - c) * 0.42).round().clamp(0.0, 255.0) as u8;
            [mix(r, 246.0), mix(g, 214.0), mix(b, 208.0)]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_wall() {
        assert_eq!(Look::from_value(&json!({ "wall": false })).unwrap().wall, None);
        let w = Look::from_value(&json!({ "wall": true })).unwrap().wall.unwrap();
        assert_eq!((w.color, w.behind, w.lamp), (Wall::COLOR, Wall::BEHIND, None));
        assert_eq!(Look::from_value(&json!({ "wall": "#ffffff" })).unwrap().wall.unwrap().color, [255, 255, 255]);
        let w = Look::from_value(&json!({ "wall": { "behind": 0.4, "lamp": [0.1, 0.3, 0.6], "softness": 0 } })).unwrap().wall.unwrap();
        assert_eq!((w.behind, w.lamp, w.softness), (0.4, Some([0.1, 0.3, 0.6]), 0.0));
        let w = Look::from_value(&json!({ "wall": { "at": [0, 0, -0.2], "facing": [0, 0, 2] } })).unwrap().wall.unwrap();
        assert_eq!((w.at, w.facing), (Some([0.0, 0.0, -0.2]), [0.0, 0.0, 2.0]));
        for bad in [json!(3), json!({ "lamp": [1, 2] }), json!({ "facing": [0, 0, 0] }), json!({ "behind": 9 }), json!({ "colour": "#fff" })] {
            assert!(Look::from_value(&json!({ "wall": bad })).unwrap_err().contains("wall"), "{bad}");
        }
    }

    #[test]
    fn reads_names_hex_and_defaults() {
        let l = Look::from_value(&json!({ "style": "toon", "skin": "#a0522d" })).unwrap();
        assert_eq!(l.style, Style::Toon);
        assert_eq!(l.skin, [0xa0, 0x52, 0x2d]);
        assert_eq!(l.outline, 3.0);
        assert_eq!(Look::from_value(&json!("clay")).unwrap().style, Style::Clay);
        assert_eq!(Look::from_value(&json!({ "skin": "dark" })).unwrap().style, Style::Realistic);
    }

    #[test]
    fn rejects_typos() {
        assert!(Look::from_value(&json!({ "style": "anime" })).unwrap_err().contains("realistic, toon"));
        assert!(Look::from_value(&json!({ "skin": "#12345" })).is_err());
        assert!(Look::from_value(&json!({ "skin": "beige" })).is_err());
        assert!(Look::from_value(&json!({ "outlines": 2 })).unwrap_err().contains("unknown look key"));
        assert!(Look::from_value(&json!({ "motion_blur": 2.5 })).is_err());
    }

    #[test]
    fn flag_and_merge() {
        let f = Look::parse_flag("spiderverse,skin=tan,outline=6,motion_blur=4").unwrap();
        let l = Look::from_value(&f).unwrap();
        assert_eq!((l.style, l.skin, l.outline, l.motion_blur), (Style::Spiderverse, TONES[2].1, 6.0, 4));
        assert!(Look::parse_flag("toon,6").is_err());
        let m = Look::merge(Some(&json!({ "style": "toon", "skin": "dark" })), Some(&json!({ "style": "arcane" }))).unwrap();
        assert_eq!(m, json!({ "style": "arcane", "skin": "dark" }));
    }

    #[test]
    fn arms() {
        assert_eq!(Look::default().arm, Arm::Bare);
        assert_eq!(Look::from_value(&json!({ "arm": "fade" })).unwrap().arm, Arm::Fade);
        let l = Look::from_value(&json!({ "style": "toon", "arm": { "sleeve": "#334477", "length": 0.1 } })).unwrap();
        assert_eq!(l.arm, Arm::Sleeve { color: [0x33, 0x44, 0x77], length: 0.1 });
        assert_eq!(l.arm.to_value(), json!({ "sleeve": [0x33, 0x44, 0x77], "length": 0.1 }));
        assert_eq!(Look::from_value(&json!({ "arm": { "sleeve": "tan" } })).unwrap().arm, Arm::Sleeve { color: TONES[2].1, length: Arm::SLEEVE_LENGTH });
        for bad in [json!("sleeve"), json!({ "sleeve": "#33447" }), json!({ "sleeve": "#334477", "length": 0.5 }), json!({ "sleeve": "#334477", "lenght": 0.1 }), json!(3)] {
            assert!(Look::from_value(&json!({ "arm": bad })).unwrap_err().contains("arm"), "{bad}");
        }
        assert_eq!(Look::from_value(&Look::parse_flag("toon,arm=fade").unwrap()).unwrap().arm, Arm::Fade);
        // A person's own arm, over the file's.
        let people = json!({ "ana": { "look": { "arm": { "sleeve": "#aa3322" } } }, "ben": { "look": { "skin": "dark" } } });
        let base = Look::from_value(&json!({ "arm": "fade" })).unwrap();
        assert!(matches!(base.for_person(Some(&people), "ana").arm, Arm::Sleeve { .. }));
        assert_eq!(base.for_person(Some(&people), "ben").arm, Arm::Fade);
        assert!(!Arm::Bare.hides_end() && Arm::Fade.hides_end());
    }

    #[test]
    fn poses_and_clips_carry_it() {
        let pose = json!({ "version": 1, "hands": { "right": {} }, "look": { "style": "clay", "skin": "brown" } });
        assert_eq!(crate::solve_value(&pose).unwrap().pose.look, Some(json!({ "style": "clay", "skin": "brown" })));
        let bad = json!({ "version": 1, "hands": { "right": {} }, "look": { "style": "watercolor" } });
        assert!(crate::solve_value(&bad).is_err());
        let clip = json!({ "version": 1, "look": "toon", "keys": [{ "t": 0, "hands": { "right": {} } }] });
        let clip = crate::clip::Clip::from_value(&clip).unwrap();
        assert_eq!(clip.sample(0.2).unwrap().pose["look"], json!("toon"));
        let bad = json!({ "version": 1, "look": { "skin": "teal" }, "keys": [{ "t": 0, "hands": { "right": {} } }] });
        assert!(crate::clip::Clip::from_value(&bad).is_err());
    }
}
