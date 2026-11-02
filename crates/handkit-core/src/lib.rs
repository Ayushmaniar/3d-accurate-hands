//! Handkit core: an anatomically constrained hand and forearm rig that agents pose in joint
//! angles. `solve` turns a pose file into world-space geometry; `query` checks it.

pub mod check;
pub mod clip;
pub mod collide;
pub mod explain;
pub mod gestures;
pub mod goals;
pub mod grip;
pub mod look;
pub mod params;
pub mod pose;
pub mod query;
pub mod rig;
pub mod solve;

use glam::DVec3;
use serde_json::{Value, json};

use pose::PoseFile;
use rig::HandGeom;

pub struct Solved {
    pub pose: PoseFile,
    pub hands: Vec<HandGeom>,
}

/// Parses a pose file and builds every hand in it.
pub fn solve(text: &str) -> Result<Solved, String> {
    let (pose, mirrors) = pose::parse(text)?;
    build(pose, mirrors)
}

/// Like `solve`, from already-parsed JSON.
pub fn solve_value(v: &Value) -> Result<Solved, String> {
    let (pose, mirrors) = pose::parse_value(v.clone())?;
    build(pose, mirrors)
}

fn build(pose: PoseFile, mirrors: pose::Mirrors) -> Result<Solved, String> {
    let version = pose.version;
    if let Some(v) = version {
        if v != 1 {
            return Err(format!("version {v} is not supported (this build reads version 1)"));
        }
    }
    let mut hands: Vec<HandGeom> = Vec::new();
    // A hand that mirrors another is built after it, from its placement.
    let (plain, mirrored): (Vec<_>, Vec<_>) = pose.hands.iter().partition(|(id, _)| mirrors.source(id).is_none());
    for (id, hp) in plain.into_iter().chain(mirrored) {
        let other = mirrors.source(id).and_then(|s| hands.iter().find(|h| h.id == *s));
        let mut hp = hp.clone();
        hp.scale = pose::person_scale(pose.people.as_ref(), &id.person);
        let geom = rig::build_hand(id, &hp, version, other)?;
        hands.push(geom);
    }
    let order = pose::hand_order(pose.hands.iter().map(|(id, _)| id));
    hands.sort_by_key(|h| order.iter().position(|id| *id == h.id));
    Ok(Solved { pose, hands })
}

fn arr(p: DVec3) -> Value {
    let r = |x: f64| (x * 1e5).round() / 1e5;
    json!([r(p.x), r(p.y), r(p.z)])
}

impl Solved {
    pub fn query(&self) -> Result<query::Report, String> {
        let mut report = query::analyze(&self.pose, &self.hands)?;
        report.goals = goals::statuses(self)?;
        Ok(report)
    }

    /// Everything a preview renderer draws.
    pub fn geometry(&self) -> Value {
        let mut g = json!({
            "camera": self.pose.camera,
            "look": self.pose.look,
            "capsules": self.hands.iter().flat_map(|h| &h.capsules).map(|c| json!({
                "a": arr(c.a), "b": arr(c.b), "r": c.r, "side": c.side.name(), "hand": &*c.hand, "label": c.label(), "arm": c.seg == rig::Seg::Arm,
            })).collect::<Vec<_>>(),
            "nails": self.hands.iter().flat_map(|h| h.nails.iter().map(move |n| json!({
                "pos": arr(n.pos), "up": arr(n.up), "out": arr(n.out), "w": n.w, "side": h.side.name(), "hand": &*h.name,
            }))).collect::<Vec<_>>(),
            "sites": self.hands.iter().flat_map(|h| &h.sites).map(|s| json!({"name": s.name, "pos": arr(s.pos)})).collect::<Vec<_>>(),
            "probes": self.pose.probes,
        });
        if let Some(p) = &self.pose.people {
            g["people"] = p.clone();
        }
        g
    }

    /// One hand's joints, its mesh's name and its corrective weights (see `bones`).
    fn hand_bones(h: &HandGeom) -> (serde_json::Map<String, Value>, String, Value) {
        let r = |x: f64| (x * 1e6).round() / 1e6;
        let mut joints = serde_json::Map::new();
        for (i, l) in h.locals().iter().enumerate() {
            let mut j = json!({
                "t": [r(l.translation.x), r(l.translation.y), r(l.translation.z)],
                "r": [r(l.rotation.x), r(l.rotation.y), r(l.rotation.z), r(l.rotation.w)],
            });
            // A smaller or larger person's hand: the root joint scales the whole skeleton.
            if h.bones[i].parent.is_none() && h.scale != 1.0 {
                j["s"] = json!(r(h.scale));
            }
            joints.insert(h.joint_name(i), j);
        }
        let suffix = if h.side == pose::Side::Left { "L" } else { "R" };
        (joints, format!("skin_{suffix}"), json!(h.corrective_weights().iter().map(|w| r(*w)).collect::<Vec<_>>()))
    }

    /// What a Handkit hand mesh needs per frame: `joints`, every skeleton joint's transform
    /// relative to its parent by glTF joint name ("index_pip_R": `t` translation in m, `r`
    /// rotation quaternion [x, y, z, w]), and `morph`, the corrective shape weights of each
    /// hand mesh ("skin_R": [..], rig::CORRECTIVES order), for the default person's hands; and
    /// `hands`, the same per hand by its name ("right", "ana.left": { joints, morph }), for
    /// poses with more people (one mesh instance per hand).
    pub fn bones(&self) -> Value {
        let mut joints = serde_json::Map::new();
        let mut morph = serde_json::Map::new();
        let mut hands = serde_json::Map::new();
        for h in &self.hands {
            let (j, skin, w) = Self::hand_bones(h);
            if h.id.is_default() {
                joints.extend(j.clone());
                morph.insert(skin.clone(), w.clone());
            }
            let mut m = serde_json::Map::new();
            m.insert(skin, w);
            hands.insert(h.name.to_string(), json!({ "joints": j, "morph": m }));
        }
        json!({ "joints": joints, "morph": morph, "hands": hands })
    }

    /// Every bone's joint position and axes: `along` points down the bone, `palmar` is the side
    /// it bends toward.
    pub fn fk_text(&self) -> String {
        let f = |p: DVec3| format!("[{:.3} {:.3} {:.3}]", p.x, p.y, p.z);
        let mut s = String::from("bone: joint position (m) | along | palmar (bend direction)\n");
        for h in &self.hands {
            for b in &h.bones {
                s.push_str(&format!("{}.{}: {} | {} | {}\n", h.name, b.name, f(b.origin), f(b.along), f(b.palmar)));
            }
            for site in &h.sites {
                s.push_str(&format!("site {}: {} facing {}\n", site.name, f(site.pos), f(site.normal)));
            }
            for d in rig::Digit::ALL {
                let name = format!("{}.{}", h.name, d.name());
                if let Ok((base, dir)) = self.finger_ray(&name) {
                    s.push_str(&format!("aim line {name}: from {} along {}\n", f(base), f(dir)));
                }
            }
        }
        s
    }
}
