//! Fingerspelling: a word as a clip of the ASL manual alphabet (`asl_a` ... `asl_z` in the
//! gesture library), with natural timing, a bounce for double letters, and J and Z traced.

use glam::DVec3;
use serde_json::{Value, json};

use crate::pose;

/// How `spell` times and frames the letters.
#[derive(Clone, Debug)]
pub struct SpellOptions {
    /// Letters per second (natural fingerspelling is about 3 to 4).
    pub rate: f64,
    /// The fraction of each letter's time it is held still (the rest is the move into it).
    pub hold: f64,
    /// Where the wrist sits for an upright letter, in meters.
    pub wrist: [f64; 3],
    /// Seconds from the start of the clip to the first letter (the hand rises into it).
    pub lead_in: f64,
    /// The clip's look (`handkit explain looks`), e.g. `{"style": "toon"}`.
    pub look: Option<Value>,
    /// The video's [width, height].
    pub size: [u32; 2],
}

impl Default for SpellOptions {
    fn default() -> Self {
        SpellOptions { rate: 3.5, hold: 0.4, wrist: [0.0, -0.03, 0.0], lead_in: 0.6, look: None, size: [1920, 1080] }
    }
}

/// The front camera `spell` frames its clips with (the hand at the default wrist).
pub fn camera() -> Value {
    json!({ "position": [0.04, 0.05, 0.66], "target": [0.04, 0.03, 0.0], "fov": 28 })
}

/// A word (letters and spaces) as a whole clip: the hand rises from rest into the first letter,
/// spells, holds the last letter and relaxes.
pub fn spell(text: &str, o: &SpellOptions) -> Result<Value, String> {
    let rest = rest_hand(o.wrist);
    let mut keys = vec![json!({ "t": 0, "hands": { "right": rest.clone() } })];
    let end = spell_keys(text, o.lead_in, o, &mut keys)?;
    let mut last = json!({ "t": r3(end + 0.9), "move": 0.6, "hands": { "right": rest } });
    last["hands"]["right"]["place"]["wrist"] = json!(v3(DVec3::from(o.wrist) + DVec3::new(0.0, -0.05, 0.0)));
    keys.push(last);
    let mut clip = json!({
        "version": 1,
        "name": format!("spell_{}", text.to_ascii_lowercase().replace(' ', "_")),
        "description": format!("\"{}\" fingerspelled in ASL, right hand, palm to the viewer (`handkit spell`).", text.to_ascii_uppercase()),
        "fps": 30,
        "size": o.size,
        "camera": camera(),
        "keys": keys,
        "duration": r3(end + 1.4),
    });
    if let Some(l) = &o.look {
        clip["look"] = l.clone();
    }
    Ok(clip)
}

/// Appends the keys that spell `text` to `keys`, the first letter arriving at `t0`, and returns
/// the time the last letter has been held (to start the next thing from). The hand must already
/// be in the clip (a previous key); its wrist moves around `o.wrist`.
pub fn spell_keys(text: &str, t0: f64, o: &SpellOptions, keys: &mut Vec<Value>) -> Result<f64, String> {
    if !(0.5..=10.0).contains(&o.rate) {
        return Err(format!("rate {}: letters per second, 0.5 to 10", o.rate));
    }
    if !(0.0..=0.9).contains(&o.hold) {
        return Err(format!("hold {}: a fraction of each letter's time, 0 to 0.9", o.hold));
    }
    let letters: Vec<char> = text.trim().chars().map(|c| c.to_ascii_lowercase()).collect();
    if letters.is_empty() {
        return Err("nothing to spell".into());
    }
    if let Some(c) = letters.iter().find(|c| !(c.is_ascii_lowercase() || **c == ' ')) {
        return Err(format!("cannot fingerspell {c:?}: letters A to Z and spaces only"));
    }
    let dt = 1.0 / o.rate;
    let base = DVec3::from(o.wrist);
    let mut t = t0;
    let mut prev: Option<(char, Value)> = None;
    let mut first = true;
    let mut last_hand: Option<Value> = None;
    let mut last_place: Option<Value> = Some(json!({ "wrist": v3(base), "palm_facing": [0, 0, 1], "fingers_toward": [0, 1, 0] }));
    for &c in &letters {
        if c == ' ' {
            t += dt;
            prev = None;
            continue;
        }
        let place = letter_place(c, base)?;
        let hand = letter_hand(c, &place)?;
        let double = prev.as_ref().is_some_and(|(pc, _)| *pc == c);
        let mut way = if double { None } else { last_hand.as_ref().and_then(|h| thumb_pass(h, &hand, c)) };
        // A half turn of the hand (upright to G or H and back) goes the way a forearm turns:
        // through the palm facing the signer's left, the fingers half way.
        if let Some(lp) = last_place.as_ref().filter(|lp| angle(&lp["palm_facing"], &place["palm_facing"]) > 120.0) {
            let fingers = (vec_of(&lp["fingers_toward"]) + vec_of(&place["fingers_toward"])).try_normalize().unwrap_or(DVec3::Y);
            let palm = (DVec3::X - fingers * fingers.x).try_normalize().unwrap_or(DVec3::Z);
            let mut w = way.unwrap_or_else(|| hand.clone());
            w["place"] = json!({ "wrist": v3((wrist_of(lp) + wrist_of(&place)) * 0.5), "palm_facing": v3(palm), "fingers_toward": v3(fingers) });
            for k in ["wrist_flex", "wrist_dev"] {
                w[k] = json!(0.0);
            }
            way = Some(w);
        }
        let mut gap = dt;
        if let Some((pc, pplace)) = &prev {
            // A big turn of the hand (into G, H, P, Q, C, O and back), or the thumb going around
            // or under the fingers, takes a little longer.
            let turn = angle(&pplace["palm_facing"], &place["palm_facing"]).max(angle(&pplace["fingers_toward"], &place["fingers_toward"]));
            if turn > 50.0 || matches!(pc, 'j' | 'z') {
                gap += 0.4 * dt;
            }
        }
        if way.is_some() {
            gap += 0.4 * dt;
        }
        if !first {
            t += gap;
        }
        let mv = if first { 0.45 } else { (gap * (1.0 - o.hold)).max(0.08) };
        if double {
            // A double letter: a small bounce down and a slide toward the signer's right.
            let mut dip = place.clone();
            dip["wrist"] = json!(v3(wrist_of(&place) + DVec3::new(-0.008, -0.018, 0.0)));
            keys.push(json!({ "t": r3(t - mv * 0.5), "move": r3(mv * 0.5), "pass": true, "hands": { "right": letter_hand(c, &dip)? } }));
            let mut slid = place.clone();
            slid["wrist"] = json!(v3(wrist_of(&place) + DVec3::new(-0.018, 0.0, 0.0)));
            keys.push(json!({ "t": r3(t), "move": r3(mv * 0.5), "hands": { "right": letter_hand(c, &slid)? } }));
        } else {
            match way {
                // The thumb crosses the curled fingers (or tucks under them): they open a little
                // on the way and the thumb lifts, so it passes instead of sliding through; or
                // the hand turns over.
                Some(way) => {
                    keys.push(json!({ "t": r3(t - mv * 0.4), "move": r3(mv * 0.6), "pass": true, "hands": { "right": way } }));
                    keys.push(json!({ "t": r3(t), "move": r3(mv * 0.4), "hands": { "right": hand.clone() } }));
                }
                None => keys.push(json!({ "t": r3(t), "move": r3(mv), "hands": { "right": hand.clone() } })),
            }
        }
        last_hand = Some(hand);
        last_place = Some(place.clone());
        match c {
            'j' => t = trace_j(t, dt, &place, keys),
            'z' => t = trace_z(t, dt, &place, keys),
            _ => {}
        }
        prev = Some((c, place));
        first = false;
    }
    Ok(r3(t + dt * o.hold.max(0.3)))
}

/// A waypoint between two letters' hands when the thumb has far to go past curled fingers (or
/// tucks under them, into E, M, N, T): the thumb halfway and lifted off the palm, the curled
/// fingers half open, everything else at the new letter.
fn thumb_pass(from: &Value, to: &Value, c: char) -> Option<Value> {
    let keys = ["cmc_flex", "cmc_abd", "cmc_rot", "mcp_flex", "ip_flex"];
    let f = |h: &Value, k: &str| h["thumb"][k].as_f64().unwrap_or(0.0);
    let travel: f64 = keys.iter().map(|k| (f(from, k) - f(to, k)).abs()).sum();
    let curled = |h: &Value| ["index", "middle", "ring", "pinky"].iter().filter(|d| h[**d]["mcp_flex"].as_f64().unwrap_or(0.0) > 45.0).count();
    if !(matches!(c, 'e' | 'm' | 'n' | 't') || (travel > 45.0 && curled(from).max(curled(to)) >= 2)) {
        return None;
    }
    let tuck = matches!(c, 'e' | 'm' | 'n' | 't');
    let mut way = to.clone();
    if !tuck {
        for k in keys {
            way["thumb"][k] = json!(r3((f(from, k) + f(to, k)) / 2.0));
        }
        way["thumb"]["cmc_abd"] = json!(r3(((f(from, "cmc_abd") + f(to, "cmc_abd")) / 2.0 + 15.0).min(70.0)));
    } else if travel > 45.0 {
        // Around the index's side: out toward the thumb side and lifted, then in.
        way["thumb"]["cmc_abd"] = json!(r3((f(from, "cmc_abd").max(f(to, "cmc_abd")) + 10.0).min(70.0)));
        way["thumb"]["cmc_flex"] = json!(r3(f(from, "cmc_flex").min(f(to, "cmc_flex")) - 15.0));
        way["thumb"]["mcp_flex"] = json!(r3(f(from, "mcp_flex").min(f(to, "mcp_flex"))));
    }
    // Tucking: the thumb goes to its place first, under fingers held open, which then close.
    let open = if tuck { [25.0, 25.0, 15.0] } else { [40.0, 40.0, 25.0] };
    for d in ["index", "middle", "ring", "pinky"] {
        let bent = |h: &Value| h[d]["mcp_flex"].as_f64().unwrap_or(0.0) > 45.0;
        if bent(from) || bent(to) {
            for (k, v) in ["mcp_flex", "pip_flex", "dip_flex"].into_iter().zip(open) {
                way[d][k] = json!(v);
            }
        }
    }
    Some(way)
}

/// J: the pinky draws a J: the hand drops while it turns, palm ending toward the signer's left.
fn trace_j(t: f64, dt: f64, start: &Value, keys: &mut Vec<Value>) -> f64 {
    let w = wrist_of(start);
    let steps = [
        (0.16, DVec3::new(0.0, -0.03, 0.0), [0.75, 0.0, 0.65], [0.0, 1.0, 0.05]),
        (0.34, DVec3::new(0.025, -0.035, 0.0), [0.9, 0.0, -0.45], [0.0, 1.0, 0.1]),
    ];
    let hold = dt * 0.3;
    let mut tt = t + hold;
    for (i, (d, off, palm, fingers)) in steps.iter().enumerate() {
        tt = t + hold + d;
        let place = json!({ "wrist": v3(w + *off), "palm_facing": palm, "fingers_toward": fingers });
        let mut key = json!({ "t": r3(tt), "move": r3(if i == 0 { *d } else { d - steps[0].0 }), "hands": { "right": letter_hand('j', &place).unwrap() } });
        if i == 0 {
            key["pass"] = json!(true);
        }
        keys.push(key);
    }
    tt
}

/// Z: the index tip draws a Z as the signer writes it (mirrored for the viewer): along the top
/// toward the signer's right, diagonally down and back, along the bottom.
fn trace_z(t: f64, dt: f64, start: &Value, keys: &mut Vec<Value>) -> f64 {
    let w = wrist_of(start);
    let (x, y) = (0.035, 0.03);
    // The start shape is drawn at the top left (the signer's left, +X).
    keys.last_mut().unwrap()["hands"]["right"]["place"]["wrist"] = json!(v3(w + DVec3::new(x, y, 0.0)));
    let corners = [DVec3::new(-x, y, 0.0), DVec3::new(x, -y, 0.0), DVec3::new(-x, -y, 0.0)];
    let mut tt = t + dt * 0.2;
    for c in corners {
        tt += 0.2;
        let mut place = start.clone();
        place["wrist"] = json!(v3(w + c));
        keys.push(json!({ "t": r3(tt), "move": 0.17, "ease": "smooth", "hands": { "right": letter_hand('z', &place).unwrap() } }));
    }
    tt
}

/// A letter's hand for a clip key: the gesture's joint values with every value written (so
/// nothing carries over from the letter before), at `place`. It carries no goals: the gesture's
/// values already meet them, and a clip lets a goal go (or closes it) with a small snap.
pub fn letter_hand(c: char, place: &Value) -> Result<Value, String> {
    let g = pose::gesture_value(&format!("asl_{c}"))?;
    let h = &g["hands"]["right"];
    let num = |v: &Value, k: &str| v.get(k).and_then(Value::as_f64).unwrap_or(0.0);
    let mut hand = json!({ "place": place });
    for k in ["twist", "wrist_flex", "wrist_dev", "palm_cup"] {
        hand[k] = json!(num(h, k));
    }
    let th = &h["thumb"];
    hand["thumb"] = json!({});
    for k in ["cmc_flex", "cmc_abd", "cmc_rot", "mcp_flex", "ip_flex"] {
        hand["thumb"][k] = json!(num(th, k));
    }
    for f in ["index", "middle", "ring", "pinky"] {
        let d = &h[f];
        let pip = num(d, "pip_flex");
        let dip = d.get("dip_flex").and_then(Value::as_f64).unwrap_or(pip * 2.0 / 3.0);
        hand[f] = json!({ "mcp_abd": num(d, "mcp_abd"), "mcp_roll": num(d, "mcp_roll"), "mcp_flex": num(d, "mcp_flex"), "pip_flex": pip, "dip_flex": r3(dip) });
    }
    Ok(hand)
}

/// The letter's placement from its gesture, moved so the wrist sits around `base`. Sideways (G,
/// H) and downward (P, Q) letters are shifted so the hand stays about where an upright one is.
fn letter_place(c: char, base: DVec3) -> Result<Value, String> {
    let g = pose::gesture_value(&format!("asl_{c}"))?;
    let mut place = g["hands"]["right"]["place"].clone();
    if !place.is_object() {
        return Err(format!("asl_{c} has no placement"));
    }
    let shift = match c {
        'g' | 'h' => DVec3::new(-0.1, 0.06, 0.0),
        'p' | 'q' => DVec3::new(-0.1, 0.1, 0.0),
        _ => DVec3::ZERO,
    };
    place["wrist"] = json!(v3(wrist_of(&place) + base + shift));
    Ok(place)
}

/// The relaxed hand a word starts and ends from, a little below the letters.
fn rest_hand(wrist: [f64; 3]) -> Value {
    json!({
        "from": "relaxed",
        "place": { "wrist": v3(DVec3::from(wrist) + DVec3::new(0.0, -0.04, 0.0)), "palm_facing": [0.2, 0, 1], "fingers_toward": [0, 1, 0.1] },
    })
}

fn wrist_of(place: &Value) -> DVec3 {
    let a = |i: usize| place["wrist"][i].as_f64().unwrap_or(0.0);
    DVec3::new(a(0), a(1), a(2))
}

fn vec_of(v: &Value) -> DVec3 {
    let a = |i: usize| v[i].as_f64().unwrap_or(0.0);
    DVec3::new(a(0), a(1), a(2)).normalize_or_zero()
}

fn angle(a: &Value, b: &Value) -> f64 {
    vec_of(a).dot(vec_of(b)).clamp(-1.0, 1.0).acos().to_degrees()
}

fn r3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

fn v3(v: DVec3) -> [f64; 3] {
    [r3(v.x), r3(v.y), r3(v.z)]
}
