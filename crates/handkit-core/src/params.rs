//! Every joint parameter: its key, limits and meaning. The single source for limit checks,
//! `handkit explain` and the agent spec.

pub struct Param {
    pub key: &'static str,
    pub min: f64,
    pub max: f64,
    pub doc: &'static str,
}

const fn p(key: &'static str, min: f64, max: f64, doc: &'static str) -> Param {
    Param { key, min, max, doc }
}

pub const PARAMS: &[Param] = &[
    p("twist", -100.0, 100.0, "Turns the hand about the forearm's long axis. +90 makes the palm face the side the thumb pointed to at rest (right hand: +X, the thumb then points to -Z)."),
    p("wrist_flex", -70.0, 80.0, "Bends the hand toward the palm side; negative bends it toward the back of the hand."),
    p("wrist_dev", -25.0, 35.0, "Bends the hand sideways toward the thumb (radial); negative toward the pinky. Applied before wrist_flex."),
    p("palm_cup", 0.0, 25.0, "Hollows the palm: the pinky knuckle comes 0.03*sin(cup) m forward and turns `cup` degrees toward the thumb, the ring knuckle half that. Use in fists, grips and thumb-to-pinky contact."),
    p("thumb.cmc_flex", -30.0, 50.0, "Swings the whole thumb in the palm plane toward the pinky side (across the palm). Negative swings it further out."),
    p("thumb.cmc_abd", -10.0, 75.0, "Lifts the whole thumb out of the palm plane toward the palm side, like opening the hand around a can."),
    p("thumb.cmc_rot", -15.0, 60.0, "Rotates the thumb about its own long axis so the pad turns further toward the fingers (opposition). Applied after cmc_flex and cmc_abd."),
    p("thumb.mcp_flex", -15.0, 65.0, "Bends the thumb's middle joint, curling the thumb toward its pad."),
    p("thumb.ip_flex", -20.0, 85.0, "Bends the thumb's tip joint, curling toward its pad."),
    p("index.mcp_abd", -15.0, 25.0, "Spreads the index away from the middle finger (toward the thumb); adds to the 4 deg rest fan."),
    p("middle.mcp_abd", -15.0, 15.0, "Tilts the middle finger toward the index side; negative toward the ring."),
    p("ring.mcp_abd", -15.0, 20.0, "Spreads the ring finger away from the middle finger (toward the pinky); adds to its -4 deg rest fan."),
    p("pinky.mcp_abd", -15.0, 35.0, "Spreads the pinky away from the ring finger; adds to its -9 deg rest fan."),
    p("index.mcp_roll", -10.0, 20.0, "Rolls the finger about its own length at the knuckle, turning the pad (and the direction the finger curls) toward the thumb; negative away. Real fingers roll a little as they curl toward the thumb, which is how a fingertip meets the thumb pad face to face."),
    p("middle.mcp_roll", -10.0, 20.0, "Rolls the finger about its own length at the knuckle, turning the pad (and the direction the finger curls) toward the thumb; negative away. Real fingers roll a little as they curl toward the thumb, which is how a fingertip meets the thumb pad face to face."),
    p("ring.mcp_roll", -10.0, 25.0, "Rolls the finger about its own length at the knuckle, turning the pad (and the direction the finger curls) toward the thumb; negative away. Real fingers roll a little as they curl toward the thumb, which is how a fingertip meets the thumb pad face to face."),
    p("pinky.mcp_roll", -10.0, 30.0, "Rolls the finger about its own length at the knuckle, turning the pad (and the direction the finger curls) toward the thumb; negative away. Real fingers roll a little as they curl toward the thumb, which is how a fingertip meets the thumb pad face to face."),
    p("index.mcp_flex", -30.0, 95.0, FLEX_MCP),
    p("index.pip_flex", -5.0, 110.0, FLEX_PIP),
    p("index.dip_flex", -10.0, 85.0, FLEX_DIP),
    p("middle.mcp_flex", -30.0, 95.0, FLEX_MCP),
    p("middle.pip_flex", -5.0, 110.0, FLEX_PIP),
    p("middle.dip_flex", -10.0, 85.0, FLEX_DIP),
    p("ring.mcp_flex", -30.0, 95.0, FLEX_MCP),
    p("ring.pip_flex", -5.0, 110.0, FLEX_PIP),
    p("ring.dip_flex", -10.0, 85.0, FLEX_DIP),
    p("pinky.mcp_flex", -30.0, 95.0, FLEX_MCP),
    p("pinky.pip_flex", -5.0, 110.0, FLEX_PIP),
    p("pinky.dip_flex", -10.0, 85.0, FLEX_DIP),
];

const FLEX_MCP: &str = "Bends the finger at the knuckle toward the palm; negative hyperextends.";
const FLEX_PIP: &str = "Bends the middle joint toward the palm.";
const FLEX_DIP: &str = "Bends the fingertip joint toward the palm. In a version 1 file, left out it follows 2/3 of pip_flex (the tendon coupling of real fingers).";

pub fn find(key: &str) -> Option<&'static Param> {
    PARAMS.iter().find(|p| p.key == key)
}
