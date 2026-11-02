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
