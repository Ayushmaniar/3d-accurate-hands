//! The gesture library: pose files under `gestures/`, embedded at build time.

include!(concat!(env!("OUT_DIR"), "/gestures.rs"));

pub fn get(name: &str) -> Option<&'static str> {
    GESTURES.iter().find(|(n, _)| *n == name).map(|(_, text)| *text)
}

/// (name, description) for every gesture.
pub fn list() -> Vec<(&'static str, String)> {
    GESTURES
        .iter()
        .map(|(name, text)| {
            let desc = serde_json::from_str::<serde_json::Value>(text)
                .ok()
                .and_then(|v| v.get("description").and_then(|d| d.as_str()).map(str::to_owned))
                .unwrap_or_default();
            (*name, desc)
        })
        .collect()
}
