//! The pose file format. Joint values are degrees, positions meters. Unknown keys are errors,
//! so a typo never silently does nothing.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Right,
    Left,
}

impl Side {
    pub fn name(self) -> &'static str {
        match self {
            Side::Right => "right",
            Side::Left => "left",
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PoseFile {
    /// 1 turns on the default couplings (dip follows pip). Files without it are read as the
    /// joint-angle experiment wrote them.
    pub version: Option<u32>,
    #[serde(default)]
    pub hands: Hands,
    /// Optional perspective camera for previews: { position, target, fov }.
    pub camera: Option<Value>,
    /// Named world points; `query` reports how well each finger aims at them.
    #[serde(default)]
    pub probes: std::collections::BTreeMap<String, [f64; 3]>,
    /// Pairs of site names whose surface gap `query` reports, e.g. ["right.thumb.pad", "right.index.pad"].
    #[serde(default)]
    pub checks: Vec<[String; 2]>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hands {
    pub right: Option<HandPose>,
    pub left: Option<HandPose>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandPose {
    pub place: Option<Place>,
    /// Older spelling of `place.elbow`.
    pub elbow: Option<[f64; 3]>,
    /// Older spelling of `place.forearm_rotation_deg`.
    pub forearm_rotation_deg: Option<[f64; 3]>,
    pub twist: Option<f64>,
    pub wrist_flex: Option<f64>,
    pub wrist_dev: Option<f64>,
    pub palm_cup: Option<f64>,
    pub thumb: Option<ThumbPose>,
    pub index: Option<FingerPose>,
    pub middle: Option<FingerPose>,
    pub ring: Option<FingerPose>,
    pub pinky: Option<FingerPose>,
}

/// Either the hand (wrist position, palm and finger directions) or the forearm (elbow and an
/// Euler rotation of the rest pose).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Place {
    pub wrist: Option<[f64; 3]>,
    pub palm_facing: Option<[f64; 3]>,
    pub fingers_toward: Option<[f64; 3]>,
    pub elbow: Option<[f64; 3]>,
    pub forearm_rotation_deg: Option<[f64; 3]>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FingerPose {
    pub mcp_flex: Option<f64>,
    pub mcp_abd: Option<f64>,
    pub pip_flex: Option<f64>,
    pub dip_flex: Option<f64>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThumbPose {
    pub cmc_flex: Option<f64>,
    pub cmc_abd: Option<f64>,
    pub cmc_rot: Option<f64>,
    pub mcp_flex: Option<f64>,
    pub ip_flex: Option<f64>,
}

impl HandPose {
    pub fn has_placement(&self) -> bool {
        self.place.is_some() || self.elbow.is_some() || self.forearm_rotation_deg.is_some()
    }
}

/// Parses a pose file. A hand written as `{"mirror": "right", ...}` starts from a copy of the
/// right hand's values (and, unless it has its own placement, its mirrored placement); the
/// returned flags say which hands mirror the other.
pub fn parse(text: &str) -> Result<(PoseFile, Mirrors), String> {
    let mut v: Value = serde_json::from_str(text).map_err(|e| format!("invalid JSON: {e}"))?;
    let mut mirrors = Mirrors::default();
    if let Some(hands) = v.get_mut("hands").and_then(Value::as_object_mut) {
        for (side, other) in [("left", "right"), ("right", "left")] {
            let Some(from) = hands.get(side).and_then(|h| h.get("mirror")).cloned() else { continue };
            if from.as_str() != Some(other) {
                return Err(format!("hands.{side}.mirror must be \"{other}\""));
            }
            let base = hands.get(other).cloned().ok_or(format!("hands.{side} mirrors \"{other}\", which is missing"))?;
            if base.get("mirror").is_some() {
                return Err("both hands mirror each other".into());
            }
            let mut own = hands[side].clone();
            own.as_object_mut().unwrap().remove("mirror");
            let has_own_place = ["place", "elbow", "forearm_rotation_deg"].iter().any(|k| own.get(*k).is_some());
            let mut merged = base;
            if !has_own_place {
                for k in ["place", "elbow", "forearm_rotation_deg"] {
                    merged.as_object_mut().unwrap().remove(k);
                }
            }
            merge(&mut merged, own);
            hands.insert(side.into(), merged);
            match side {
                "left" => mirrors.left = !has_own_place,
                _ => mirrors.right = !has_own_place,
            }
        }
    }
    let pose: PoseFile = serde_json::from_value(v).map_err(|e| format!("pose file: {e}"))?;
    Ok((pose, mirrors))
}

/// Which hands take the mirror image of the other hand's placement.
#[derive(Clone, Copy, Debug, Default)]
pub struct Mirrors {
    pub left: bool,
    pub right: bool,
}

fn merge(base: &mut Value, over: Value) {
    match (base, over) {
        (Value::Object(b), Value::Object(o)) => {
            for (k, v) in o {
                match b.get_mut(&k) {
                    Some(slot) if slot.is_object() && v.is_object() => merge(slot, v),
                    _ => {
                        b.insert(k, v);
                    }
                }
            }
        }
        (b, o) => *b = o,
    }
}
