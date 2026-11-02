//! The Handkit core for JavaScript. Every function takes and returns JSON text, so the same
//! pose files work in Rust, the CLI and the browser.

use wasm_bindgen::prelude::*;

fn solved(pose: &str) -> Result<handkit_core::Solved, JsError> {
    handkit_core::solve(pose).map_err(|e| JsError::new(&e))
}

/// Joint transforms and corrective shape weights for a pose file:
/// `{"joints": {"index_pip_R": {"t": [..], "r": [x, y, z, w]}}, "morph": {"skin_R": [..]},
/// "hands": {"ana.right": {"joints": .., "morph": ..}}}`. Apply them with `handRig(gltf.scene)`
/// from `@handkit/three` (one mesh per hand: `handBones(bones, name)`).
#[wasm_bindgen]
pub fn bones(pose: &str) -> Result<String, JsError> {
    Ok(solved(pose)?.bones().to_string())
}

/// The `handkit query` report as JSON: goals, limits, contacts, penetrations.
#[wasm_bindgen]
pub fn query(pose: &str) -> Result<String, JsError> {
    let report = solved(pose)?.query().map_err(|e| JsError::new(&e))?;
    Ok(report.json(false).to_string())
}

/// `handkit solve`: the pose file with its goals met, as JSON text.
#[wasm_bindgen]
pub fn solve(pose: &str) -> Result<String, JsError> {
    let raw: serde_json::Value = serde_json::from_str(pose).map_err(|e| JsError::new(&e.to_string()))?;
    let (out, _) = handkit_core::solve::solve_goals(&raw).map_err(|e| JsError::new(&e))?;
    Ok(handkit_core::pose::to_text(&out))
}

/// A library gesture as a pose file.
#[wasm_bindgen]
pub fn gesture(name: &str) -> Result<String, JsError> {
    let v = handkit_core::pose::gesture_value(name).map_err(|e| JsError::new(&e))?;
    Ok(handkit_core::pose::to_text(&v))
}

/// A `look` (an object or a bare style name, see `handkit explain looks`) checked and with
/// every default filled in: `{"style": "toon", "skin": [r, g, b], "nail": [r, g, b],
/// "nail_given": false, "outline": 3, "motion_blur": 1, "shutter": 180, "halftone": 8,
/// "misregister": 3, "texture": 1, "subsurface": 0.35}` (colours sRGB bytes). `"null"` is the
/// default look. `HandkitLook` in `@handkit/three` draws it.
#[wasm_bindgen]
pub fn look(value: &str) -> Result<String, JsError> {
    let v: serde_json::Value = serde_json::from_str(value).map_err(|e| JsError::new(&e.to_string()))?;
    let l = if v.is_null() { handkit_core::look::Look::default() } else { handkit_core::look::Look::from_value(&v).map_err(|e| JsError::new(&e))? };
    Ok(look_json(&l).to_string())
}

fn look_json(l: &handkit_core::look::Look) -> serde_json::Value {
    serde_json::json!({
        "style": l.style.name(),
        "skin": l.skin,
        "nail": l.nail_color(),
        "nail_given": l.nail.is_some(),
        "outline": l.outline,
        "motion_blur": l.motion_blur,
        "shutter": l.shutter,
        "halftone": l.halftone.unwrap_or(8.0),
        "misregister": l.misregister.unwrap_or(3.0),
        "texture": l.texture.unwrap_or(1.0),
        "subsurface": l.subsurface.unwrap_or(0.35),
    })
}

/// A clip (keyed poses over time), parsed and solved once; sample it per frame.
#[wasm_bindgen]
pub struct Clip(handkit_core::clip::Clip);

#[wasm_bindgen]
impl Clip {
    #[wasm_bindgen(constructor)]
    pub fn new(text: &str) -> Result<Clip, JsError> {
        handkit_core::clip::Clip::parse(text).map(Clip).map_err(|e| JsError::new(&e))
    }

    #[wasm_bindgen(getter)]
    pub fn duration(&self) -> f64 {
        self.0.duration
    }

    #[wasm_bindgen(getter)]
    pub fn fps(&self) -> f64 {
        self.0.fps
    }

    /// The clip's hands by name, as JSON: `["right", "left", "ana.right", ...]`.
    #[wasm_bindgen(getter)]
    pub fn hands(&self) -> String {
        serde_json::to_string(&self.0.hands().iter().map(|h| h.name()).collect::<Vec<_>>()).unwrap()
    }

    /// The clip's camera (`{"position", "target", "fov"}`) as JSON text, `null` without one.
    #[wasm_bindgen(getter)]
    pub fn camera(&self) -> String {
        self.0.camera.as_ref().map_or("null".into(), |c| c.to_string())
    }

    /// The clip's `look` with defaults filled in (see `look`), JSON text.
    #[wasm_bindgen(getter)]
    pub fn look(&self) -> Result<String, JsError> {
        let l = match &self.0.look {
            Some(v) => handkit_core::look::Look::from_value(v).map_err(|e| JsError::new(&e))?,
            None => handkit_core::look::Look::default(),
        };
        Ok(look_json(&l).to_string())
    }

    /// The clip's `size` (`[width, height]`) as JSON text, `null` when it gives none (16:9).
    #[wasm_bindgen(getter)]
    pub fn size(&self) -> String {
        self.0.size.as_ref().map_or("null".into(), |s| s.to_string())
    }

    /// Each hand's look by hand name, as JSON: the clip's `look` (see `look`) in the colours of
    /// the hand's person (`people.<person>.look` sets their skin and nail; the style is the
    /// clip's): `{"right": {..}, "ana.right": {"style": .., "skin": [r, g, b], ..}}`.
    pub fn looks(&self) -> Result<String, JsError> {
        let base = match &self.0.look {
            Some(v) => handkit_core::look::Look::from_value(v).map_err(|e| JsError::new(&e))?,
            None => handkit_core::look::Look::default(),
        };
        let mut out = serde_json::Map::new();
        for id in self.0.hands() {
            out.insert(id.name(), look_json(&base.for_person(self.0.people.as_ref(), &id.person)));
        }
        Ok(serde_json::Value::Object(out).to_string())
    }

    /// The pose file at `t` seconds, as JSON text.
    pub fn sample(&self, t: f64) -> Result<String, JsError> {
        Ok(self.0.sample(t).map_err(|e| JsError::new(&e))?.pose.to_string())
    }

    /// Joint transforms and corrective weights at `t`, for `handRig` (see `bones`).
    pub fn bones(&self, t: f64) -> Result<String, JsError> {
        let f = self.0.sample(t).map_err(|e| JsError::new(&e))?;
        let solved = handkit_core::solve_value(&f.pose).map_err(|e| JsError::new(&e))?;
        Ok(solved.bones().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(text: String) -> serde_json::Value {
        serde_json::from_str(&text).unwrap()
    }

    #[test]
    fn looks_get_their_defaults() {
        let l = json(look("\"toon\"").ok().unwrap());
        assert_eq!(l["style"], "toon");
        assert_eq!(l["outline"], 3.0);
        assert_eq!(l["skin"], serde_json::json!([0xd8, 0xa6, 0x8a]));
        assert_eq!(l["nail_given"], false);
        let l = json(look(r##"{"style": "spiderverse", "skin": "dark", "nail": "#ffffff"}"##).ok().unwrap());
        assert_eq!((l["halftone"].as_f64(), l["misregister"].as_f64()), (Some(8.0), Some(3.0)));
        assert_eq!(l["nail"], serde_json::json!([255, 255, 255]));
        let l = json(look("null").ok().unwrap());
        assert_eq!((l["style"].as_str(), l["subsurface"].as_f64()), (Some("realistic"), Some(0.35)));
    }

    #[test]
    fn clips_give_camera_and_look() {
        let clip = Clip::new(include_str!("../../../clips/demo_basics.json")).ok().unwrap();
        assert_eq!(json(clip.camera())["fov"], 35);
        assert_eq!(json(clip.look().ok().unwrap())["style"], "realistic");
    }

    #[test]
    fn clips_give_each_hands_look() {
        let clip = Clip::new(include_str!("../../../clips/demo_four_hands.json")).ok().unwrap();
        let hands = json(clip.hands());
        let looks = json(clip.looks().ok().unwrap());
        let skins: Vec<_> = hands.as_array().unwrap().iter().map(|h| looks[h.as_str().unwrap()]["skin"].clone()).collect();
        assert!(skins.iter().all(|s| s.is_array()));
        assert!(skins.iter().any(|s| *s != skins[0]), "the two people's skins differ");
    }
}
