// Forward kinematics for the experiment hand. Everything is computed for a RIGHT hand in the
// forearm frame, then mirrored in X for a left hand and placed in the world.
import * as THREE from 'three';

const D = Math.PI / 180;

export const LIMITS = {
  twist: [-100, 100], wrist_flex: [-70, 80], wrist_dev: [-25, 35], palm_cup: [0, 25],
  finger: { mcp_flex: [-30, 95], pip_flex: [-5, 110], dip_flex: [-10, 85] },
  mcp_abd: { index: [-15, 25], middle: [-15, 15], ring: [-15, 20], pinky: [-15, 35] },
  thumb: { cmc_flex: [-20, 50], cmc_abd: [-10, 75], cmc_rot: [-15, 60], mcp_flex: [-15, 65], ip_flex: [-20, 85] },
};

const FOREARM = 0.26;
// MCP joint (knuckle) position in the hand frame, metacarpal base, rest fan angle, phalanx lengths.
const FINGERS = {
  index:  { mcp: [0.030, 0.093], base: [0.012, 0.010], fan: 4,  len: [0.040, 0.025, 0.020], r: 0.0095 },
  middle: { mcp: [0.010, 0.096], base: [0.004, 0.012], fan: 0,  len: [0.045, 0.028, 0.021], r: 0.0098 },
  ring:   { mcp: [-0.010, 0.091], base: [-0.006, 0.012], fan: -4, len: [0.042, 0.027, 0.021], r: 0.0092 },
  pinky:  { mcp: [-0.029, 0.083], base: [-0.015, 0.010], fan: -9, len: [0.033, 0.019, 0.018], r: 0.0082 },
};
const THUMB = { cmc: [0.020, 0.018, 0.008], len: [0.046, 0.034, 0.027], r: [0.0135, 0.0105, 0.0095] };

const rx = (a) => new THREE.Matrix4().makeRotationX(a * D);
const ry = (a) => new THREE.Matrix4().makeRotationY(a * D);
const rz = (a) => new THREE.Matrix4().makeRotationZ(a * D);
const tr = (x, y, z) => new THREE.Matrix4().makeTranslation(x, y, z);
const mul = (...ms) => ms.reduce((acc, m) => acc.multiply(m), new THREE.Matrix4());
const origin = (m) => new THREE.Vector3().setFromMatrixPosition(m);
const axis = (m, i) => new THREE.Vector3().setFromMatrixColumn(m, i).normalize();

function check(violations, name, value, [lo, hi]) {
  if (value < lo - 0.01 || value > hi + 0.01) violations.push(`${name}=${value} outside [${lo}, ${hi}]`);
}

// Returns segments (capsules) and nails in world space, plus joint-limit violations.
export function buildHand(p, side) {
  const v = [];
  const g = (o, k) => (o && typeof o[k] === 'number' ? o[k] : 0);
  check(v, 'twist', g(p, 'twist'), LIMITS.twist);
  check(v, 'wrist_flex', g(p, 'wrist_flex'), LIMITS.wrist_flex);
  check(v, 'wrist_dev', g(p, 'wrist_dev'), LIMITS.wrist_dev);
  check(v, 'palm_cup', g(p, 'palm_cup'), LIMITS.palm_cup);

  const e = p.forearm_rotation_deg || [0, 0, 0];
  const elbow = p.elbow || [0, 0, 0];
  const world = new THREE.Matrix4().compose(
    new THREE.Vector3(...elbow),
    new THREE.Quaternion().setFromEuler(new THREE.Euler(e[0] * D, e[1] * D, e[2] * D, 'XYZ')),
    new THREE.Vector3(1, 1, 1),
  );
  const mirror = new THREE.Matrix4().makeScale(side === 'left' ? -1 : 1, 1, 1);
  const toWorld = mul(world, mirror);
  const P = (m) => origin(m).applyMatrix4(toWorld);
  const Dir = (m, i) => axis(m, i).transformDirection(toWorld);

  const segs = [];
  const nails = [];
  const seg = (a, b, r, part, kind) => segs.push({ a: a.toArray(), b: b.toArray(), r, part, kind, side });

  // Forearm: elbow at origin, wrist at +Y. Twist turns the hand about the forearm axis.
  const wrist = mul(tr(0, FOREARM, 0), ry(g(p, 'twist')));
  seg(P(new THREE.Matrix4()), P(wrist), 0.028, 'forearm', 'arm');
  const hand = mul(wrist, rz(-g(p, 'wrist_dev')), rx(g(p, 'wrist_flex')));
  const cup = g(p, 'palm_cup');

  for (const [name, f] of Object.entries(FINGERS)) {
    const q = p[name] || {};
    for (const k of ['mcp_flex', 'pip_flex', 'dip_flex']) check(v, `${name}.${k}`, g(q, k), LIMITS.finger[k]);
    check(v, `${name}.mcp_abd`, g(q, 'mcp_abd'), LIMITS.mcp_abd[name]);
    const cupK = name === 'pinky' ? 1 : name === 'ring' ? 0.5 : 0;
    const base = mul(hand, tr(f.base[0], f.base[1], 0));
    let m = mul(hand, tr(f.mcp[0], f.mcp[1], 0.03 * Math.sin(cup * cupK * D)), ry(cup * cupK));
    seg(P(base), P(m), 0.0125, 'palm', 'metacarpal');
    // Positive abduction spreads away from the middle finger (middle: toward the index side).
    const abdSign = name === 'ring' || name === 'pinky' ? 1 : -1;
    m = mul(m, rz(-f.fan + abdSign * g(q, 'mcp_abd')));
    const flex = [g(q, 'mcp_flex'), g(q, 'pip_flex'), g(q, 'dip_flex')];
    const kinds = ['proximal', 'middle', 'distal'];
    for (let i = 0; i < 3; i++) {
      m = mul(m, rx(flex[i]));
      const next = mul(m, tr(0, f.len[i], 0));
      seg(P(m), P(next), f.r * (1 - 0.1 * i), name, kinds[i]);
      if (i === 2) nails.push({ pos: P(mul(m, tr(0, f.len[2] * 0.6, -f.r * 0.75))).toArray(), up: Dir(m, 1).toArray(), out: Dir(m, 2).negate().toArray(), w: f.r * 1.5, part: name, side });
      m = next;
    }
  }

  const t = p.thumb || {};
  for (const k of Object.keys(LIMITS.thumb)) check(v, `thumb.${k}`, g(t, k), LIMITS.thumb[k]);
  // Rest: thumb leans 35 deg toward the thumb side and 10 deg out of the palm, pad turned toward
  // the index. Flex swings it in the palm plane toward the pinky, abduction lifts it out of the
  // palm plane, rotation turns it about its own axis (opposition).
  let m = mul(hand, tr(...THUMB.cmc), rz(-35 + g(t, 'cmc_flex')), rx(10 + g(t, 'cmc_abd')), ry(-55 - g(t, 'cmc_rot')));
  const tflex = [0, g(t, 'mcp_flex'), g(t, 'ip_flex')];
  const tkinds = ['metacarpal', 'proximal', 'distal'];
  for (let i = 0; i < 3; i++) {
    m = mul(m, rx(tflex[i]));
    const next = mul(m, tr(0, THUMB.len[i], 0));
    seg(P(m), P(next), THUMB.r[i], 'thumb', tkinds[i]);
    if (i === 2) nails.push({ pos: P(mul(m, tr(0, THUMB.len[2] * 0.6, -THUMB.r[2] * 0.75))).toArray(), up: Dir(m, 1).toArray(), out: Dir(m, 2).negate().toArray(), w: THUMB.r[2] * 1.5, part: 'thumb', side });
    m = next;
  }
  return { segs, nails, violations: v.map((s) => `${side}.${s}`) };
}

// Closest distance between segments p1-q1 and p2-q2.
function segDist(p1, q1, p2, q2) {
  const d1 = q1.clone().sub(p1), d2 = q2.clone().sub(p2), r = p1.clone().sub(p2);
  const a = d1.dot(d1), e = d2.dot(d2), f = d2.dot(r);
  let s, t;
  if (a <= 1e-12 && e <= 1e-12) return p1.distanceTo(p2);
  if (a <= 1e-12) { s = 0; t = THREE.MathUtils.clamp(f / e, 0, 1); }
  else {
    const c = d1.dot(r);
    if (e <= 1e-12) { t = 0; s = THREE.MathUtils.clamp(-c / a, 0, 1); }
    else {
      const b = d1.dot(d2), den = a * e - b * b;
      s = den !== 0 ? THREE.MathUtils.clamp((b * f - c * e) / den, 0, 1) : 0;
      t = (b * s + f) / e;
      if (t < 0) { t = 0; s = THREE.MathUtils.clamp(-c / a, 0, 1); }
      else if (t > 1) { t = 1; s = THREE.MathUtils.clamp((b - c) / a, 0, 1); }
    }
  }
  return p1.clone().addScaledVector(d1, s).distanceTo(p2.clone().addScaledVector(d2, t));
}

const DIGITS = ['thumb', 'index', 'middle', 'ring', 'pinky'];

// Interpenetration between different digits / hands, and fingertip surface gaps.
export function analyze(hands) {
  const segs = hands.flatMap((h) => h.segs);
  const V = (a) => new THREE.Vector3(...a);
  const pen = [];
  for (let i = 0; i < segs.length; i++) {
    for (let j = i + 1; j < segs.length; j++) {
      const A = segs[i], B = segs[j];
      if (A.side === B.side) {
        if (A.part === B.part) continue;
        const aFinger = DIGITS.includes(A.part) && A.kind !== 'metacarpal';
        const bFinger = DIGITS.includes(B.part) && B.kind !== 'metacarpal';
        if (!aFinger && !bFinger) continue;
        // A proximal phalanx always touches the palm at its knuckle.
        if ((A.kind === 'proximal' && !bFinger) || (B.kind === 'proximal' && !aFinger)) continue;
      }
      const depth = A.r + B.r - segDist(V(A.a), V(A.b), V(B.a), V(B.b));
      if (depth > 0.004) pen.push(`${A.side}.${A.part}.${A.kind} x ${B.side}.${B.part}.${B.kind}: ${(depth * 1000).toFixed(1)} mm deep`);
    }
  }
  const tips = {};
  for (const s of segs) if (s.kind === 'distal') tips[`${s.side}.${s.part}`] = { p: V(s.b), r: s.r };
  const tipPos = Object.fromEntries(Object.entries(tips).map(([k, t]) => [k, t.p.toArray().map((x) => +x.toFixed(3))]));
  const gaps = {};
  const keys = Object.keys(tips);
  for (let i = 0; i < keys.length; i++) for (let j = i + 1; j < keys.length; j++) {
    const [ka, kb] = [keys[i], keys[j]];
    const sameHand = ka.split('.')[0] === kb.split('.')[0];
    if (sameHand && !ka.endsWith('thumb') && !kb.endsWith('thumb')) continue;
    gaps[`${ka}~${kb}`] = +((tips[ka].p.distanceTo(tips[kb].p) - tips[ka].r - tips[kb].r) * 1000).toFixed(1);
  }
  return { penetrations: pen, fingertip_world_m: tipPos, fingertip_surface_gaps_mm: gaps };
}
