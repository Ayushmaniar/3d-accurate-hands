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
    /// Contacts and aims that `handkit solve` makes true by changing joint values.
    #[serde(default)]
    pub goals: Vec<Goal>,
    /// A gesture library entry's name and one-line description.
    pub name: Option<String>,
    pub description: Option<String>,
}

/// One goal: `touch` two sites or segments, or `aim` a finger (e.g. "right.index") `at` a point.
/// `vary` lists what the solver may change (see `handkit explain`); left out, it picks the joints
/// of the digits involved.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Goal {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub touch: Option<[String; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aim: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<[f64; 3]>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vary: Vec<String>,
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
    pub mcp_roll: Option<f64>,
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

/// Parses a pose file. See `resolve` for `from` and `mirror`.
pub fn parse(text: &str) -> Result<(PoseFile, Mirrors), String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("invalid JSON: {e}"))?;
    parse_value(v)
}

pub fn parse_value(v: Value) -> Result<(PoseFile, Mirrors), String> {
    let (v, mirrors) = resolve(v)?;
    let pose: PoseFile = serde_json::from_value(v).map_err(|e| format!("pose file: {e}"))?;
    Ok((pose, mirrors))
}

/// Expands presets and mirrors into plain values.
/// - `"from": "<gesture>"` at the top starts from a whole library pose; on a hand it starts from
///   that gesture's hand (its touch goals come along, renamed to this hand; its aims, which
///   point at world positions, do not).
/// - A hand written as `{"mirror": "right", ...}` starts from a copy of the right hand's values
///   and, unless it has its own placement, takes the mirror image of its placement.
/// Keys written in the file always win, and goals from presets are added to the file's own.
pub fn resolve(mut v: Value) -> Result<(Value, Mirrors), String> {
    if !v.is_object() {
        return Err("a pose file is a JSON object".into());
    }
    let mut extra_goals: Vec<Value> = Vec::new();
    if let Some(name) = v.as_object_mut().unwrap().remove("from") {
        let name = name.as_str().ok_or("\"from\" must be a gesture name")?;
        let mut base = gesture_value(name)?;
        let base_obj = base.as_object_mut().unwrap();
        for k in ["name", "description"] {
            base_obj.remove(k);
        }
        if let Some(Value::Array(g)) = base_obj.remove("goals") {
            extra_goals.extend(g);
        }
        merge(&mut base, v);
        v = base;
    }
    if let Some(hands) = v.get_mut("hands").and_then(Value::as_object_mut) {
        for side in ["right", "left"] {
            let Some(hand) = hands.get_mut(side).and_then(Value::as_object_mut) else { continue };
            let Some(name) = hand.remove("from") else { continue };
            let name = name.as_str().ok_or(format!("hands.{side}.from must be a gesture name"))?;
            let preset = gesture_value(name)?;
            let ph = preset.get("hands").and_then(Value::as_object).cloned().unwrap_or_default();
            let (from_side, base) = match ph.get(side) {
                Some(h) => (side, h.clone()),
                None if ph.len() == 1 => {
                    let (s, h) = ph.iter().next().unwrap();
                    (if s == "right" { "right" } else { "left" }, h.clone())
                }
                None => return Err(format!("gesture \"{name}\" has no {side} hand")),
            };
            let mut base = base;
            for k in ["place", "elbow", "forearm_rotation_deg", "mirror"] {
                base.as_object_mut().map(|o| o.remove(k));
            }
            merge(&mut base, Value::Object(std::mem::take(hand)));
            *hand = base.as_object().cloned().unwrap_or_default();
            // An aim is at a world point, which only makes sense with the preset's placement.
            for g in preset.get("goals").and_then(Value::as_array).cloned().unwrap_or_default() {
                if g.get("aim").is_some() {
                    continue;
                }
                let text = g.to_string().replace(&format!("\"{from_side}."), &format!("\"{side}."));
                extra_goals.push(serde_json::from_str(&text).unwrap());
            }
        }
    }
    if !extra_goals.is_empty() {
        let obj = v.as_object_mut().unwrap();
        let goals = obj.entry("goals").or_insert(Value::Array(Vec::new()));
        let arr = goals.as_array_mut().ok_or("\"goals\" must be a list")?;
        for g in extra_goals {
            if !arr.contains(&g) {
                arr.push(g);
            }
        }
    }
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
    Ok((v, mirrors))
}

/// A gesture library entry as JSON, by name ("ok" or "gestures/ok").
pub fn gesture_value(name: &str) -> Result<Value, String> {
    let key = name.trim_start_matches("gestures/").trim_end_matches(".json");
    let text = crate::gestures::get(key).ok_or_else(|| {
        format!("unknown gesture \"{name}\"; run `handkit gestures` for the list")
    })?;
    serde_json::from_str(text).map_err(|e| format!("gesture {key}: {e}"))
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

/// A pose file as text: indented, with anything that fits on one line kept on one line.
pub fn to_text(v: &Value) -> String {
    fn inline(v: &Value) -> String {
        match v {
            Value::Array(a) => format!("[{}]", a.iter().map(inline).collect::<Vec<_>>().join(", ")),
            Value::Object(o) if o.is_empty() => "{}".into(),
            Value::Object(o) => format!("{{ {} }}", o.iter().map(|(k, v)| format!("{}: {}", Value::from(k.as_str()), inline(v))).collect::<Vec<_>>().join(", ")),
            _ => v.to_string(),
        }
    }
    fn block(v: &Value, indent: usize, used: usize) -> String {
        let one = inline(v);
        if indent + used + one.len() <= 110 || !(v.is_object() || v.is_array()) {
            return one;
        }
        let pad = " ".repeat(indent + 2);
        let items: Vec<String> = match v {
            Value::Object(o) => o.iter().map(|(k, x)| {
                let key = format!("{}: ", Value::from(k.as_str()));
                format!("{pad}{key}{}", block(x, indent + 2, key.len()))
            }).collect(),
            Value::Array(a) => a.iter().map(|x| format!("{pad}{}", block(x, indent + 2, 0))).collect(),
            _ => unreachable!(),
        };
        let (open, close) = if v.is_object() { ("{", "}") } else { ("[", "]") };
        format!("{open}\n{}\n{}{close}", items.join(",\n"), " ".repeat(indent))
    }
    block(v, 0, 0) + "\n"
}
