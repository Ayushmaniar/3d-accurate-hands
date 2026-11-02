//! The pose file format. Joint values are degrees, positions meters. Unknown keys are errors,
//! so a typo never silently does nothing.

use glam::DVec3;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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

    pub fn parse(s: &str) -> Option<Side> {
        match s {
            "right" => Some(Side::Right),
            "left" => Some(Side::Left),
            _ => None,
        }
    }

    pub fn other(self) -> Side {
        match self {
            Side::Right => Side::Left,
            Side::Left => Side::Right,
        }
    }
}

/// A hand: whose (`person`, "" for the default person) and which. Written "right", "left",
/// "ana.right", "ana.left"; every site, segment and pattern starts with it ("ana.right.index.tip").
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HandId {
    pub person: String,
    pub side: Side,
}

impl HandId {
    pub fn new(person: &str, side: Side) -> HandId {
        HandId { person: person.to_string(), side }
    }

    /// The default person's hand: "right" or "left".
    pub fn of(side: Side) -> HandId {
        HandId { person: String::new(), side }
    }

    pub fn is_default(&self) -> bool {
        self.person.is_empty()
    }

    pub fn name(&self) -> String {
        if self.person.is_empty() { self.side.name().to_string() } else { format!("{}.{}", self.person, self.side.name()) }
    }

    /// The same person's other hand.
    pub fn other(&self) -> HandId {
        HandId { person: self.person.clone(), side: self.side.other() }
    }

    /// A whole hand name: "right", "left", "<person>.right" or "<person>.left".
    pub fn parse(s: &str) -> Result<HandId, String> {
        match split_hand(s) {
            Some((id, "")) => Ok(id),
            _ => Err(format!("\"{s}\" is not a hand: write right, left, or <person>.right / <person>.left (person names: lowercase letters, digits and _)")),
        }
    }
}

impl std::fmt::Display for HandId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name())
    }
}

/// A person's name: lowercase letters, digits and `_`, not "right" or "left".
pub fn valid_person(name: &str) -> bool {
    !name.is_empty() && Side::parse(name).is_none() && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Splits a name that starts with a hand into the hand and the rest:
/// "ana.right.index.tip" -> (ana.right, "index.tip"), "left.thumb" -> (left, "thumb"),
/// "right" -> (right, "").
pub fn split_hand(name: &str) -> Option<(HandId, &str)> {
    let (first, rest) = name.split_once('.').unwrap_or((name, ""));
    if let Some(side) = Side::parse(first) {
        return Some((HandId::of(side), rest));
    }
    if !valid_person(first) {
        return None;
    }
    let (second, rest) = rest.split_once('.').unwrap_or((rest, ""));
    Some((HandId::new(first, Side::parse(second)?), rest))
}

/// The hand a site, segment or pattern name starts with, as written ("ana.right" of
/// "ana.right.index.tip"); the whole name when it does not start with one.
pub fn hand_of(name: &str) -> String {
    split_hand(name).map(|(id, _)| id.name()).unwrap_or_else(|| name.to_string())
}

/// Checks a top-level `people`: { "<person>": { "look": {..} } }.
pub fn check_people(v: &Value) -> Result<(), String> {
    let obj = v.as_object().ok_or("\"people\" is an object: { \"<person>\": { \"look\": {..} } }")?;
    for (name, p) in obj {
        if !valid_person(name) {
            return Err(format!("people: \"{name}\" is not a person name (lowercase letters, digits and _; not right or left)"));
        }
        let po = p.as_object().ok_or(format!("people.{name} is an object: {{ \"look\": {{..}} }}"))?;
        for (k, x) in po {
            match k.as_str() {
                "look" => {
                    crate::look::Look::from_value(x).map_err(|e| format!("people.{name}: {e}"))?;
                }
                _ => return Err(format!("people.{name}: unknown field \"{k}\" (fields: look)")),
            }
        }
    }
    Ok(())
}

/// The look a person's hands are drawn with: the top-level look with the person's own over it.
pub fn person_look(look: Option<&Value>, people: Option<&Value>, person: &str) -> Option<Value> {
    let own = people.and_then(|p| p.get(person)).and_then(|p| p.get("look"));
    crate::look::Look::merge(look, own)
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
    /// How renderers draw the hands: { style, skin, nail, outline, ... } (`look::Look`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look: Option<Value>,
    /// Per person (the prefix of "ana.right"): { "ana": { "look": {..} } }, a look over the
    /// top-level one for that person's hands.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub people: Option<Value>,
    /// The frame's [width, height] in pixels, for `place.screen` (default 16:9).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<[f64; 2]>,
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

/// The hands of a pose, in the order the file lists them.
#[derive(Clone, Debug, Default)]
pub struct Hands(pub Vec<(HandId, HandPose)>);

impl Hands {
    pub fn get(&self, id: &HandId) -> Option<&HandPose> {
        self.0.iter().find(|(h, _)| h == id).map(|(_, p)| p)
    }

    pub fn right(&self) -> Option<&HandPose> {
        self.get(&HandId::of(Side::Right))
    }

    pub fn left(&self) -> Option<&HandPose> {
        self.get(&HandId::of(Side::Left))
    }

    pub fn iter(&self) -> impl Iterator<Item = &(HandId, HandPose)> {
        self.0.iter()
    }
}

/// The order hands are built and listed in: the default person's right and left, then
/// everyone else's hands in the order given.
pub fn hand_order<'a>(ids: impl Iterator<Item = &'a HandId>) -> Vec<HandId> {
    let mut v: Vec<HandId> = ids.cloned().collect();
    v.sort_by_key(|h| match (h.is_default(), h.side) {
        (true, Side::Right) => 0,
        (true, Side::Left) => 1,
        _ => 2,
    });
    v
}

impl Serialize for Hands {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut m = s.serialize_map(Some(self.0.len()))?;
        for (id, h) in &self.0 {
            m.serialize_entry(&id.name(), h)?;
        }
        m.end()
    }
}

impl<'de> Deserialize<'de> for Hands {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let map = serde_json::Map::<String, Value>::deserialize(d)?;
        let mut out = Vec::new();
        for (k, v) in map {
            let id = HandId::parse(&k).map_err(|_| serde::de::Error::custom(format!("unknown hand \"{k}\": hands are right, left, or <person>.right / <person>.left (person names: lowercase letters, digits and _)")))?;
            let h: HandPose = serde_json::from_value(v).map_err(|e| serde::de::Error::custom(format!("hands.{k}: {e}")))?;
            out.push((id, h));
        }
        Ok(Hands(out))
    }
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
    /// With `wrist`: the direction from the wrist toward the elbow. Sets wrist_flex and
    /// wrist_dev so the forearm points there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forearm_toward: Option<[f64; 3]>,
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
    if let Some(l) = &pose.look {
        crate::look::Look::from_value(l)?;
    }
    if let Some(p) = &pose.people {
        check_people(p)?;
    }
    Ok((pose, mirrors))
}

/// Expands presets and mirrors into plain values.
/// - `"from": "<gesture>"` at the top starts from a whole library pose; on a hand it starts from
///   that gesture's hand (its touch goals come along, renamed to this hand; its aims, which
///   point at world positions, do not).
/// - A hand written as `{"mirror": "right", ...}` starts from a copy of the right hand's values
///   and, unless it has its own placement, takes the mirror image of its placement.
/// - `place.screen` + `depth` become `place.wrist`, and `place.elbow_screen` becomes
///   `forearm_toward`, through the file's camera and size.
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
        let keys: Vec<String> = hands.keys().cloned().collect();
        for key in keys {
            let Ok(id) = HandId::parse(&key) else { continue };
            let side = id.side.name();
            let Some(hand) = hands.get_mut(&key).and_then(Value::as_object_mut) else { continue };
            let Some(name) = hand.remove("from") else { continue };
            let name = name.as_str().ok_or(format!("hands.{key}.from must be a gesture name"))?;
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
                let text = g.to_string().replace(&format!("\"{from_side}."), &format!("\"{key}."));
                extra_goals.push(serde_json::from_str(&text).unwrap());
            }
        }
    }
    screen_places(&mut v)?;
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
        let mirroring: Vec<(String, Value)> = hands.iter().filter_map(|(k, h)| Some((k.clone(), h.get("mirror")?.clone()))).collect();
        for (side, from) in &mirroring {
            let Ok(id) = HandId::parse(side) else { continue };
            // Any hand of the other side, of this person or another; "right" for the left hand.
            let other = from.as_str().and_then(|s| HandId::parse(s).ok()).filter(|o| o.side != id.side);
            let Some(other) = other.map(|o| o.name()) else {
                return Err(format!("hands.{side}.mirror must be \"{}\"", id.other().name()));
            };
            let base = hands.get(&other).cloned().ok_or(format!("hands.{side} mirrors \"{other}\", which is missing"))?;
            if mirroring.iter().any(|(k, _)| *k == other) {
                return Err(if mirroring.len() == 2 { "both hands mirror each other".into() } else { format!("hands.{side} mirrors \"{other}\", which mirrors a hand itself") });
            }
            let mut own = hands[side.as_str()].clone();
            own.as_object_mut().unwrap().remove("mirror");
            let has_own_place = ["place", "elbow", "forearm_rotation_deg"].iter().any(|k| own.get(*k).is_some());
            let mut merged = base;
            if !has_own_place {
                for k in ["place", "elbow", "forearm_rotation_deg"] {
                    merged.as_object_mut().unwrap().remove(k);
                }
            }
            merge(&mut merged, own);
            hands.insert(side.clone(), merged);
            if !has_own_place {
                mirrors.of.push((id, HandId::parse(&other).unwrap()));
            }
        }
    }
    Ok((v, mirrors))
}

/// A pose camera's view: position, forward, right and up, and 1 / tan(fov / 2).
pub struct View {
    pub pos: DVec3,
    pub f: DVec3,
    pub r: DVec3,
    pub u: DVec3,
    pub k: f64,
}

impl View {
    /// From a pose camera ({position, target, fov}, fov vertical in degrees, default 35).
    pub fn of(cam: &Value) -> Option<View> {
        let a = |k: &str| -> Option<DVec3> {
            let v = cam.get(k)?.as_array()?;
            Some(DVec3::new(v.first()?.as_f64()?, v.get(1)?.as_f64()?, v.get(2)?.as_f64()?))
        };
        let (pos, target) = (a("position")?, a("target")?);
        let fov = cam.get("fov").and_then(Value::as_f64).unwrap_or(35.0).to_radians();
        let f = (target - pos).normalize();
        let r = f.cross(DVec3::Y).normalize();
        Some(View { pos, f, r, u: r.cross(f), k: 1.0 / (fov / 2.0).tan() })
    }

    /// The point at `depth` meters along the view through `screen` (fractions of the frame: x 0
    /// left to 1 right, y 0 top to 1 bottom).
    pub fn unproject(&self, aspect: f64, screen: [f64; 2], depth: f64) -> DVec3 {
        let (x, y) = (2.0 * screen[0] - 1.0, 1.0 - 2.0 * screen[1]);
        self.pos + (self.f + self.r * (x * aspect / self.k) + self.u * (y / self.k)) * depth
    }
}

/// The frame aspect of a pose or clip: its "size" (width / height), default 16:9.
pub fn aspect(v: &Value) -> f64 {
    let s = v.get("size").and_then(Value::as_array);
    s.and_then(|s| Some(s.first()?.as_f64()? / s.get(1)?.as_f64()?)).filter(|a| a.is_finite() && *a > 0.0).unwrap_or(16.0 / 9.0)
}

/// Turns `place.screen` + `depth` into `place.wrist`, and `place.elbow_screen` into
/// `forearm_toward`, with the pose's camera and size.
fn screen_places(v: &mut Value) -> Result<(), String> {
    let aspect = aspect(v);
    let cam = v.get("camera").cloned();
    let Some(hands) = v.get_mut("hands").and_then(Value::as_object_mut) else { return Ok(()) };
    for (side, hand) in hands.iter_mut() {
        let Some(place) = hand.get_mut("place").and_then(Value::as_object_mut) else { continue };
        if !place.contains_key("screen") && !place.contains_key("elbow_screen") {
            continue;
        }
        let at = |e: &str| format!("hands.{side}.place: {e}");
        let view = cam.as_ref().and_then(View::of).ok_or_else(|| at("screen and elbow_screen need the pose's \"camera\" ({ position, target, fov })"))?;
        let pair = |x: &Value, k: &str| -> Result<[f64; 2], String> {
            let a: Option<Vec<f64>> = x.as_array().and_then(|a| a.iter().map(Value::as_f64).collect());
            a.filter(|a| a.len() == 2).map(|a| [a[0], a[1]]).ok_or_else(|| at(&format!("{k} is [x, y], fractions of the frame")))
        };
        if let Some(s) = place.remove("screen") {
            if place.contains_key("wrist") {
                return Err(at("give either screen + depth or wrist, not both"));
            }
            let depth = place.remove("depth").and_then(|d| d.as_f64()).ok_or_else(|| at("screen needs \"depth\": meters from the camera along its view"))?;
            let p = view.unproject(aspect, pair(&s, "screen")?, depth);
            place.insert("wrist".into(), serde_json::json!([p.x, p.y, p.z]));
        } else if place.contains_key("depth") {
            return Err(at("depth goes with screen"));
        }
        if let Some(s) = place.remove("elbow_screen") {
            if place.contains_key("forearm_toward") {
                return Err(at("give either elbow_screen or forearm_toward, not both"));
            }
            let w: Option<Vec<f64>> = place.get("wrist").and_then(Value::as_array).and_then(|a| a.iter().map(Value::as_f64).collect());
            let w = w.filter(|w| w.len() == 3).map(|w| DVec3::new(w[0], w[1], w[2])).ok_or_else(|| at("elbow_screen needs wrist or screen"))?;
            // The elbow on the line of sight through that point, a forearm's length from the
            // wrist: the farther of the two such points, or the nearest point of the line.
            let d = (view.unproject(aspect, pair(&s, "elbow_screen")?, 1.0) - view.pos).normalize();
            let b = d.dot(w - view.pos);
            let disc = b * b - (w - view.pos).length_squared() + crate::rig::FOREARM * crate::rig::FOREARM;
            let elbow = view.pos + d * (b + disc.max(0.0).sqrt());
            let t = elbow - w;
            place.insert("forearm_toward".into(), serde_json::json!([t.x, t.y, t.z]));
        }
    }
    Ok(())
}

/// A gesture library entry as JSON, by name ("ok" or "gestures/ok").
pub fn gesture_value(name: &str) -> Result<Value, String> {
    let key = name.trim_start_matches("gestures/").trim_end_matches(".json");
    let text = crate::gestures::get(key).ok_or_else(|| {
        format!("unknown gesture \"{name}\"; run `handkit gestures` for the list")
    })?;
    serde_json::from_str(text).map_err(|e| format!("gesture {key}: {e}"))
}

/// Which hands take the mirror image of another hand's placement: (the hand, the one it mirrors).
#[derive(Clone, Debug, Default)]
pub struct Mirrors {
    pub of: Vec<(HandId, HandId)>,
}

impl Mirrors {
    pub fn source(&self, id: &HandId) -> Option<&HandId> {
        self.of.iter().find(|(h, _)| h == id).map(|(_, s)| s)
    }
}

pub(crate) fn merge(base: &mut Value, over: Value) {
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
