//! How a hand is drawn: a pose's or clip's top-level `look`. Core only checks it and passes it
//! on; renderers (handkit-fframes' `look` module, the three.js preview) draw it.
//!
//! ```json
//! "look": { "style": "toon", "skin": "tan", "nail": "#e9c4b8", "outline": 3, "motion_blur": 4 }
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

/// A checked look. Fields left `None` take the style's default (the renderer's choice).
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
}

impl Default for Look {
    fn default() -> Self {
        Look::of(Style::Realistic)
    }
}

pub const KEYS: [&str; 10] = ["style", "skin", "nail", "outline", "motion_blur", "shutter", "halftone", "misregister", "texture", "subsurface"];

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

    /// This look in a person's colours: the skin and nail of their own look
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
