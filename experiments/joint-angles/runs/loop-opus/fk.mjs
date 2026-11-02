// My own guess at the rig's forward kinematics, built only from SPEC.md.
// Used to predict fingertip positions before rendering; calibrated against render reports.
import fs from 'fs';

const D = Math.PI / 180;
export const mul = (A, B) => A.map((r, i) => [0, 1, 2].map(j => r[0] * B[0][j] + r[1] * B[1][j] + r[2] * B[2][j]));
export const mv = (A, v) => A.map(r => r[0] * v[0] + r[1] * v[1] + r[2] * v[2]);
export const add = (a, b) => a.map((x, i) => x + b[i]);
export const sub = (a, b) => a.map((x, i) => x - b[i]);
export const sc = (a, s) => a.map(x => x * s);
export const norm = a => Math.hypot(...a);
export const Rx = d => { const c = Math.cos(d * D), s = Math.sin(d * D); return [[1, 0, 0], [0, c, -s], [0, s, c]]; };
export const Ry = d => { const c = Math.cos(d * D), s = Math.sin(d * D); return [[c, 0, s], [0, 1, 0], [-s, 0, c]]; };
export const Rz = d => { const c = Math.cos(d * D), s = Math.sin(d * D); return [[c, -s, 0], [s, c, 0], [0, 0, 1]]; };
const I = [[1, 0, 0], [0, 1, 0], [0, 0, 1]];

export const CFG = { lean: 25, thumbPad0: -40, thumbRotSign: 1, wristOrder: 'twist-flex-dev' };

const FING = {
  index: { at: [0.030, 0.093], L: [0.040, 0.025, 0.020], fan: 4, abdSign: 1 },
  middle: { at: [0.010, 0.096], L: [0.045, 0.028, 0.021], fan: 0, abdSign: 1 },
  ring: { at: [-0.010, 0.091], L: [0.042, 0.027, 0.021], fan: -4, abdSign: -1 },
  pinky: { at: [-0.029, 0.083], L: [0.033, 0.019, 0.018], fan: -9, abdSign: -1 },
};

// returns joints in "right-hand rest frame" relative to elbow (before mirror and forearm rotation)
function localJoints(h) {
  const tw = h.twist || 0, fl = h.wrist_flex || 0, dv = h.wrist_dev || 0;
  const W = mul(mul(Ry(tw), Rx(fl)), Rz(-dv));
  const wrist = [0, 0.26, 0];
  const P = v => add(wrist, mv(W, v));
  const out = { wrist };
  for (const [n, f] of Object.entries(FING)) {
    const j = h[n] || {};
    let R = mul(W, Rz(-(f.fan + f.abdSign * (j.mcp_abd || 0))));
    let p = P([f.at[0], f.at[1], 0]);
    const pts = [p];
    const fl = [j.mcp_flex || 0, j.pip_flex || 0, j.dip_flex || 0];
    for (let k = 0; k < 3; k++) {
      R = mul(R, Rx(fl[k]));
      p = add(p, mv(R, [0, f.L[k], 0]));
      pts.push(p);
    }
    out[n] = pts;
  }
  const t = h.thumb || {};
  // fitted from render reports (fit3.mjs): base = Rz(-35) Rx(10); pad offset -55 deg; cmc_rot turns as Ry(-rot)
  let R = mul(mul(mul(mul(mul(W, Rz(-35)), Rx(10)), Rz(t.cmc_flex || 0)), Rx(t.cmc_abd || 0)), Ry(-55 - (t.cmc_rot || 0)));
  let p = P([0.020, 0.018, 0.008]);
  const pts = [p];
  const L = [0.046, 0.034, 0.027], fl2 = [0, t.mcp_flex || 0, t.ip_flex || 0];
  for (let k = 0; k < 3; k++) { R = mul(R, Rx(fl2[k])); p = add(p, mv(R, [0, L[k], 0])); pts.push(p); }
  out.thumb = pts;
  return out;
}

export function fk(h, side) {
  const loc = localJoints(h);
  const M = side === 'left' ? v => [-v[0], v[1], v[2]] : v => v;
  const [rx, ry, rz] = h.forearm_rotation_deg || [0, 0, 0];
  const R = mul(mul(Rx(rx), Ry(ry)), Rz(rz));
  const E = h.elbow || [0, 0, 0];
  const tf = v => add(E, mv(R, M(v)));
  const out = {};
  for (const [k, v] of Object.entries(loc)) out[k] = Array.isArray(v[0]) ? v.map(tf) : tf(v);
  return out;
}

if (process.argv[1] && process.argv[1].endsWith('fk.mjs') && process.argv[2]) {
  const pose = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
  for (const [side, h] of Object.entries(pose.hands)) {
    const j = fk(h, side);
    const f = a => a.map(x => x.toFixed(3)).join(', ');
    console.log(side, 'wrist', f(j.wrist));
    for (const n of ['thumb', 'index', 'middle', 'ring', 'pinky']) console.log(' ', n.padEnd(6), 'tip', f(j[n][3]), ' base', f(j[n][0]));
  }
}
