//! The Handkit core for JavaScript. Every function takes and returns JSON text, so the same
//! pose files work in Rust, the CLI and the browser.

use wasm_bindgen::prelude::*;

fn solved(pose: &str) -> Result<handkit_core::Solved, JsError> {
    handkit_core::solve(pose).map_err(|e| JsError::new(&e))
}

/// Joint transforms and corrective shape weights for a pose file:
/// `{"joints": {"index_pip_R": {"t": [..], "r": [x, y, z, w]}}, "morph": {"skin_R": [..]}}`.
/// Apply them with `handRig(gltf.scene)` from `@handkit/three`.
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
