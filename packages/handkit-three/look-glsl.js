// GLSL for the looks (look.js): ports of crates/handkit-fframes/src/look/{img,surface,shade}.rs
// and handkit_mesh::skin's tone model, so three.js draws what fframes draws.
// three compiles them as GLSL ES 3.00 (WebGL2), with gl_FragColor and texture2D mapped; colours are linear.

/** Helpers: smoothstep that allows reversed edges (as the Rust one does), the integer hash
 * and value noise of img.rs, bit for bit. */
export const COMMON = /* glsl */ `
float ss(float e0, float e1, float x) { float t = clamp((x - e0) / (e1 - e0), 0.0, 1.0); return t * t * (3.0 - 2.0 * t); }
float lum3(vec3 c) { return dot(c, vec3(0.2126, 0.7152, 0.0722)); }
float hashi(int x, int y, int z) {
  uint h = (uint(x) * 0x8da6b343u) ^ (uint(y) * 0xd8163841u) ^ (uint(z) * 0xcb1ab31fu);
  h ^= h >> 13u;
  h *= 0x5bd1e995u;
  h ^= h >> 15u;
  return float(h & 0x00ffffffu) / 16777216.0;
}
float fadef(float t) { return t * t * (3.0 - 2.0 * t); }
float noise2(float x, float y) {
  int xi = int(floor(x)), yi = int(floor(y));
  float fx = fadef(x - floor(x)), fy = fadef(y - floor(y));
  float a = hashi(xi, yi, 7) + (hashi(xi + 1, yi, 7) - hashi(xi, yi, 7)) * fx;
  float b = hashi(xi, yi + 1, 7) + (hashi(xi + 1, yi + 1, 7) - hashi(xi, yi + 1, 7)) * fx;
  return a + (b - a) * fy;
}
float noise3(vec3 p) {
  vec3 i = floor(p);
  vec3 f = p - i;
  float fx = fadef(f.x), fy = fadef(f.y), fz = fadef(f.z);
  int xi = int(i.x), yi = int(i.y), zi = int(i.z);
  float x00 = mix(hashi(xi, yi, zi), hashi(xi + 1, yi, zi), fx);
  float x10 = mix(hashi(xi, yi + 1, zi), hashi(xi + 1, yi + 1, zi), fx);
  float x01 = mix(hashi(xi, yi, zi + 1), hashi(xi + 1, yi, zi + 1), fx);
  float x11 = mix(hashi(xi, yi + 1, zi + 1), hashi(xi + 1, yi + 1, zi + 1), fx);
  return mix(mix(x00, x10, fy), mix(x01, x11, fy), fz);
}
float fbm3(vec3 p) { return clamp(noise3(p) * 0.65 + noise3(p * 2.03 + vec3(17.3)) * 0.35, 0.0, 1.0); }
`;

/** handkit_mesh::skin: the tone model the realistic colour map is rebuilt with. */
export const TONE = /* glsl */ `
float darkness(vec3 t) { return ss(0.30, 0.04, lum3(t)); }
vec3 nailColor(vec3 t) { return mix(t * vec3(1.02, 0.9, 0.92), vec3(0.745404, 0.479320, 0.401978), 0.42 + 0.18 * darkness(t)); }
// m = (redness, nail, cavity, palm)
vec3 toneBase(vec3 tone, vec3 nail, vec4 m) {
  float dark = darkness(tone);
  vec3 back = tone * vec3(0.92, 0.95, 0.94);
  vec3 pink = tone * vec3(0.95, 0.95, 1.07);
  vec3 pale = mix(tone, vec3(0.672443, 0.341914, 0.254152), 0.55);
  vec3 palm = mix(pink, pale, 0.8 * dark);
  vec3 skin = mix(back, palm, m.a);
  skin = mix(skin, vec3(lum3(skin)), 0.2 * dark);
  skin = mix(skin, skin * vec3(1.0, 0.62, 0.68), 0.35 * m.r * (1.0 - 0.35 * dark));
  skin *= mix(vec3(1.0), vec3(0.78, 0.68, 0.66), m.b);
  return mix(skin, nail, m.g);
}
`;

export const QUAD_VS = /* glsl */ `
out vec2 vUv;
void main() { vUv = uv; gl_Position = vec4(position.xy, 0.0, 1.0); }
`;

/** The colour map for a skin tone (surface.rs tone): the baked map times the ratio of the
 * tone models at each texel's masks. */
export const TONE_MAP_FS = /* glsl */ `
${COMMON}
${TONE}
uniform sampler2D color; // sRGB texture, read linear
uniform sampler2D masks; // linear: R redness, G nail, B cavity
uniform sampler2D orm;   // B: palm
uniform vec3 tone, nail, bakedTone;
in vec2 vUv;
void main() {
  vec4 m = vec4(texture(masks, vUv).rgb, texture(orm, vUv).b);
  vec3 baked = texture(color, vUv).rgb;
  vec3 from = toneBase(bakedTone, nailColor(bakedTone), m);
  vec3 to = toneBase(tone, nail, m);
  gl_FragColor = vec4(min(baked * to / max(from, vec3(1e-4)), vec3(1.0)), 1.0);
}
`;

/** The mesh drawn into its UV atlas (bind pose), per texel: the point, normal and the skin
 * weights of the nearest corner. Mode 0 coverage, 1 arcane strokes' tint, 2 arcane stroke
 * normals, 3 clay normals. */
export const BAKE_VS = /* glsl */ `
in vec3 bp;
in vec3 bn;
in vec3 bary;
in vec4 jA; in vec4 jB; in vec4 jC; in vec4 wA; in vec4 wB; in vec4 wC;
out vec3 vP; out vec3 vN; out vec3 vBary;
flat out vec4 fjA; flat out vec4 fjB; flat out vec4 fjC; flat out vec4 fwA; flat out vec4 fwB; flat out vec4 fwC;
void main() {
  vP = bp; vN = bn; vBary = bary;
  fjA = jA; fjB = jB; fjC = jC; fwA = wA; fwB = wB; fwC = wC;
  gl_Position = vec4(position.xy * 2.0 - 1.0, 0.0, 1.0);
}
`;

export const BAKE_FS = (bones) => /* glsl */ `
${COMMON}
#define BONES ${bones}
uniform int mode;
uniform float size;
uniform mat4 ibm[BONES];
in vec3 vP; in vec3 vN; in vec3 vBary;
flat in vec4 fjA; flat in vec4 fjB; flat in vec4 fjC; flat in vec4 fwA; flat in vec4 fwB; flat in vec4 fwC;

vec4 J; vec4 W;

// surface.rs strokes: (tint 0..1, paint height in meters).
vec2 strokes(vec3 p, vec3 n) {
  float pick = noise3(p * 180.0 + vec3(3.7));
  float acc = 0.0;
  int bone = int(J.x + 0.5);
  for (int k = 0; k < 4; k++) {
    acc += W[k];
    if (pick * 0.999 < acc) { bone = int(J[k] + 0.5); break; }
  }
  mat4 m = bone < BONES ? ibm[bone] : mat4(1.0);
  vec3 q = (m * vec4(p, 1.0)).xyz;
  vec3 nn = (m * vec4(n, 0.0)).xyz;
  nn = length(nn) > 0.0 ? normalize(nn) : vec3(0.0, 0.0, 1.0);
  float wobble = (noise3(p * 220.0) - 0.5) * 0.5;
  float u, v; int plane;
  if (abs(nn.y) > 0.8 + wobble * 0.3) { u = q.x; v = q.z; plane = 2; }
  else if (abs(nn.x) > abs(nn.z) + wobble) { u = q.z; v = q.y; plane = 0; }
  else { u = q.x; v = q.y; plane = 1; }
  vec2 pp = vec2(u, v);
  float cw = 0.0034, ch = 0.0075;
  vec2 cell = vec2(floor(pp.x / cw), floor(pp.y / ch));
  int seed = bone * 3 + plane;
  float best = -1.0, tint = 0.5, height = 0.0;
  for (int dy = -1; dy <= 1; dy++) {
    for (int dx = -1; dx <= 1; dx++) {
      int ix = int(cell.x) + dx, iy = int(cell.y) + dy;
      int hx = ix + 7919 * seed;
      vec2 centre = vec2((float(ix) + hashi(hx, iy, 1)) * cw, (float(iy) + hashi(hx, iy, 2)) * ch);
      float angle = (hashi(hx, iy, 3) - 0.5) * 0.9;
      float sa = sin(angle), ca = cos(angle);
      vec2 d = pp - centre;
      vec2 local = vec2(d.x * ca - d.y * sa, d.x * sa + d.y * ca);
      float hw = cw * (0.55 + 0.35 * hashi(hx, iy, 4)), hl = ch * (0.6 + 0.5 * hashi(hx, iy, 5));
      float rag = (noise2(local.y / hl * 3.0 + hashi(hx, iy, 6) * 40.0, hashi(hx, iy, 7) * 40.0) - 0.5) * 0.35;
      float across = abs(local.x / hw) + rag;
      float along = abs(local.y / hl);
      float cover = ss(1.0, 0.85, across) * ss(1.0, 0.75, along + 0.25 * abs(local.x / hw));
      float priority = hashi(hx, iy, 8) + cover;
      if (cover > 0.35 && priority > best) {
        best = priority;
        tint = 0.5 + (hashi(hx, iy, 9) - 0.5) * 0.6;
        float bristle = sin(local.x / hw * 9.0 + noise2(local.y / hl * 2.0, hashi(hx, iy, 10) * 30.0) * 3.0);
        height = cover * (0.6 + 0.4 * bristle) * 2e-5;
      }
    }
  }
  return vec2(tint, height);
}

// surface.rs clay_height.
float clayHeight(vec3 p, vec3 n) {
  float lumps = (fbm3(p * 110.0) - 0.5) * 6e-4 + (noise3(p * 260.0 + vec3(5.0)) - 0.5) * 8e-5;
  float cell = 0.019;
  vec3 c = floor(p / cell);
  float print = 0.0, weight = 0.0;
  for (int dz = -1; dz <= 1; dz++) for (int dy = -1; dy <= 1; dy++) for (int dx = -1; dx <= 1; dx++) {
    int ix = int(c.x) + dx, iy = int(c.y) + dy, iz = int(c.z) + dz;
    if (hashi(ix * 31 + 1, iy, iz) < 0.45) continue;
    vec3 centre = (vec3(float(ix), float(iy), float(iz)) + vec3(hashi(ix * 31 + 2, iy, iz), hashi(ix * 31 + 3, iy, iz), hashi(ix * 31 + 4, iy, iz))) * cell;
    vec3 d = p - centre;
    vec3 along = d - n * dot(n, d);
    vec3 axis = vec3(hashi(ix * 31 + 5, iy, iz) - 0.5, hashi(ix * 31 + 6, iy, iz) - 0.5, hashi(ix * 31 + 7, iy, iz) - 0.5);
    axis = length(axis) > 0.0 ? normalize(axis) : vec3(1.0, 0.0, 0.0);
    float stretch = dot(along, axis) * 0.3;
    float dist = sqrt(max(dot(along, along) - stretch * stretch * (2.0 - 0.3), 0.0));
    float radius = cell * (0.32 + 0.18 * hashi(ix * 31 + 8, iy, iz));
    float w = 1.0 - ss(radius * 0.4, radius, dist);
    if (w > weight) {
      weight = w;
      float warp = (noise3(p * 400.0 + vec3(hashi(ix * 31 + 9, iy, iz) * 50.0)) - 0.5) * 0.6;
      print = sin((dist / 0.0009 + warp) * 6.28318530718) * w * (0.5 + 0.5 * hashi(ix * 31 + 10, iy, iz));
    }
  }
  return lumps + print * 1.4e-5;
}

float heightAt(vec3 p, vec3 n) { return mode == 2 ? strokes(p, n).y : clayHeight(p, n); }

void main() {
  vec3 n = normalize(vN);
  if (vBary.x >= vBary.y && vBary.x >= vBary.z) { J = fjA; W = fwA; }
  else if (vBary.y >= vBary.z) { J = fjB; W = fwB; }
  else { J = fjC; W = fwC; }
  if (mode == 0) { gl_FragColor = vec4(1.0); return; }
  if (mode == 1) { gl_FragColor = vec4(strokes(vP, n).x, 0.0, 0.0, 1.0); return; }
  // A tangent-space normal map of the height, glTF convention (surface.rs normal_map).
  vec3 dpdu = dFdx(vP) * size, dpdv = dFdy(vP) * size;
  vec3 tu = dpdu - n * dot(n, dpdu); tu = length(tu) > 0.0 ? normalize(tu) : vec3(0.0);
  vec3 tv = dpdv - n * dot(n, dpdv); tv = length(tv) > 0.0 ? normalize(tv) : vec3(0.0);
  float e = clamp(0.5 * length(dFdx(vP)), 2e-5, 5e-4);
  float gu = (heightAt(vP + tu * e, n) - heightAt(vP - tu * e, n)) / (2.0 * e);
  float gv = (heightAt(vP + tv * e, n) - heightAt(vP - tv * e, n)) / (2.0 * e);
  vec3 m = normalize(vec3(-gu, gv, 1.0));
  gl_FragColor = vec4(m * 0.5 + 0.5, 1.0);
}
`;

/** Grows covered texels (alpha 1) into empty neighbours, for filtering across chart borders. */
export const DILATE_FS = /* glsl */ `
uniform sampler2D src;
in vec2 vUv;
void main() {
  ivec2 p = ivec2(gl_FragCoord.xy);
  ivec2 n = textureSize(src, 0) - 1;
  vec4 c = texelFetch(src, p, 0);
  if (c.a < 0.5) {
    ivec2 o[4] = ivec2[4](ivec2(-1, 0), ivec2(1, 0), ivec2(0, -1), ivec2(0, 1));
    for (int k = 0; k < 4; k++) {
      vec4 q = texelFetch(src, clamp(p + o[k], ivec2(0), n), 0);
      if (q.a >= 0.5) { c = q; break; }
    }
  }
  gl_FragColor = c;
}
`;

/** Data map step 1: (cavity x covered, covered). */
export const CAVITY_FS = /* glsl */ `
uniform sampler2D masks;
uniform sampler2D cover;
in vec2 vUv;
void main() {
  float c = texelFetch(cover, ivec2(gl_FragCoord.xy), 0).a >= 0.5 ? 1.0 : 0.0;
  gl_FragColor = vec4(texture(masks, vUv).b * c, c, 0.0, 1.0);
}
`;

/** Data map step 2 (surface.rs data_pixels): R nails and crease lines (ridges of the smoothed
 * cavity), G the palm or arcane's stroke tint. */
export const DATA_FS = /* glsl */ `
${COMMON}
uniform sampler2D masks;
uniform sampler2D orm;
uniform sampler2D cover;  // a: covered (grown)
uniform sampler2D blurred; // (cavity, weight) blurred
uniform sampler2D tint;    // arcane: r tint, a covered
uniform int arcane;
in vec2 vUv;
float smoothAt(ivec2 p) {
  ivec2 n = textureSize(blurred, 0) - 1;
  vec2 b = texelFetch(blurred, min(p, n), 0).rg;
  return b.g > 1e-3 ? b.r / b.g : 0.0;
}
float pack(float nail, float line) { return nail > 0.5 ? 0.55 + 0.45 * ss(0.5, 1.0, nail) : 0.45 * min(max(line, nail * 2.0), 1.0); }
void main() {
  ivec2 p = ivec2(gl_FragCoord.xy);
  vec4 m = vec4(texture(masks, vUv).rgb, texture(orm, vUv).b);
  bool covered = texelFetch(cover, p, 0).a >= 0.5;
  float weight = texelFetch(blurred, p, 0).g;
  float line = 0.0;
  if (covered && p.x > 0 && p.y > 0) {
    float c = smoothAt(p);
    float l = smoothAt(p + ivec2(-1, 0)), r = smoothAt(p + ivec2(1, 0)), u = smoothAt(p + ivec2(0, -1)), d = smoothAt(p + ivec2(0, 1));
    float gx = (r - l) * 0.5, gy = (d - u) * 0.5;
    float hxx = r - 2.0 * c + l, hyy = d - 2.0 * c + u;
    float hxy = (smoothAt(p + ivec2(1, 1)) - smoothAt(p + ivec2(1, -1)) - smoothAt(p + ivec2(-1, 1)) + smoothAt(p + ivec2(-1, -1))) * 0.25;
    float mean = 0.5 * (hxx + hyy);
    float root = sqrt(0.25 * (hxx - hyy) * (hxx - hyy) + hxy * hxy);
    float lambda = mean - root;
    if (lambda < 0.0) {
      vec2 nn = abs(hxy) > 1e-9 ? vec2(lambda - hyy, hxy) : (hxx < hyy ? vec2(1.0, 0.0) : vec2(0.0, 1.0));
      nn /= max(length(nn), 1e-9);
      float t = -(gx * nn.x + gy * nn.y) / lambda;
      line = exp(-(t / 0.75) * (t / 0.75)) * ss(0.004, 0.02, -lambda) * ss(0.25, 0.5, c) * ss(0.85, 0.98, weight);
    }
  }
  float g = m.a;
  if (arcane == 1) { vec4 t = texelFetch(tint, p, 0); g = t.a >= 0.5 ? t.r : 0.5; }
  gl_FragColor = vec4(pack(m.g, line), g, 0.0, 1.0);
}
`;

/** A separable gaussian of a vec4 (`dir` one texel across or down), optionally converting packed
 * normals (rgb = (n * 0.5 + 0.5) * a) to (n * a, a) first. */
export const BLUR_FS = /* glsl */ `
uniform sampler2D src;
uniform vec2 dir;
uniform float sigma;
uniform int packed;
in vec2 vUv;
vec4 at(ivec2 p) {
  ivec2 n = textureSize(src, 0) - 1;
  vec4 c = texelFetch(src, clamp(p, ivec2(0), n), 0);
  return packed == 1 ? vec4(c.rgb * 2.0 - c.a, c.a) : c;
}
void main() {
  ivec2 p = ivec2(gl_FragCoord.xy);
  if (sigma < 0.3) { gl_FragColor = at(p); return; }
  int r = int(ceil(sigma * 3.0));
  vec4 sum = vec4(0.0); float wsum = 0.0;
  for (int i = -r; i <= r; i++) {
    float w = exp(-0.5 * float(i * i) / (sigma * sigma));
    sum += at(p + ivec2(dir) * i) * w; wsum += w;
  }
  gl_FragColor = sum / wsum;
}
`;

/** Ink line seeds (shade.rs edges): R the silhouette, G interior edges (normal creases and
 * depth steps, on the nearer side). y runs up here; fframes' "down" is our -y. */
export const SEEDS_FS = /* glsl */ `
uniform sampler2D normals;   // raw normals pass: a = alpha
uniform sampler2D blurredN;  // (n * a, a) blurred
uniform sampler2D data;      // (r, g, distance) * a, a
uniform int interior;
in vec2 vUv;
ivec2 N;
float alphaAt(ivec2 p) { return texelFetch(normals, p, 0).a; }
vec3 nAt(ivec2 p) { vec3 n = texelFetch(blurredN, p, 0).xyz; return length(n) > 1e-6 ? normalize(n) : vec3(0.0, 0.0, 1.0); }
float dAt(ivec2 p) { vec4 d = texelFetch(data, p, 0); return d.a > 1e-4 ? d.b / d.a : 0.0; }
void main() {
  N = textureSize(normals, 0);
  ivec2 p = ivec2(gl_FragCoord.xy);
  bool inI = alphaAt(p) >= 0.5;
  float sil = 0.0, inner = 0.0;
  vec3 ni = nAt(p); float di = dAt(p);
  ivec2 o[4] = ivec2[4](ivec2(1, 0), ivec2(0, -1), ivec2(-1, 0), ivec2(0, 1));
  for (int k = 0; k < 4; k++) {
    ivec2 q = p + o[k];
    if (q.x < 0 || q.y < 0 || q.x >= N.x || q.y >= N.y) continue;
    bool inJ = alphaAt(q) >= 0.5;
    if (inI != inJ) { sil = 1.0; continue; }
    if (interior == 1 && inI) {
      float dj = dAt(q);
      bool edge = dot(ni, nAt(q)) < 0.62 || abs(di - dj) > 0.007;
      // Right and down pairs keep ties on this side, as fframes scans them.
      bool nearer = k < 2 ? di <= dj : di < dj;
      if (edge && nearer) inner = 1.0;
    }
  }
  gl_FragColor = vec4(sil, inner, 0.0, 1.0);
}
`;

/** Exact euclidean distance to the seeds within `reach` pixels, separable: pass 0 the nearest
 * seed along the row, pass 1 the nearest over the column. */
export const EDT_FS = /* glsl */ `
uniform sampler2D src;
uniform int pass;
uniform int reach;
in vec2 vUv;
void main() {
  ivec2 p = ivec2(gl_FragCoord.xy);
  ivec2 n = textureSize(src, 0);
  const float FAR = 1e4;
  if (pass == 0) {
    vec2 best = vec2(FAR);
    for (int i = -reach; i <= reach; i++) {
      int x = p.x + i;
      if (x < 0 || x >= n.x) continue;
      vec2 s = texelFetch(src, ivec2(x, p.y), 0).rg;
      float d = abs(float(i));
      if (s.r > 0.5) best.x = min(best.x, d);
      if (s.g > 0.5) best.y = min(best.y, d);
    }
    gl_FragColor = vec4(best, 0.0, 1.0);
  } else {
    vec2 best = vec2(FAR * FAR);
    for (int j = -reach; j <= reach; j++) {
      int y = p.y + j;
      if (y < 0 || y >= n.y) continue;
      vec2 dx = texelFetch(src, ivec2(p.x, y), 0).rg;
      float dy2 = float(j * j);
      best = min(best, dx * dx + dy2);
    }
    gl_FragColor = vec4(sqrt(best), 0.0, 1.0);
  }
}
`;

/** Shared by the line and shade passes: inputs at a pixel. */
const STYLED_INPUTS = /* glsl */ `
uniform sampler2D normals;   // a: alpha
uniform sampler2D blurredN;
uniform sampler2D data;
uniform sampler2D dist;      // r silhouette, g interior distance
uniform vec2 anchor;         // pixels, y down
uniform float ppm, scale, outline, texture_, halftone, misreg;
uniform int style;           // 1 toon, 2 spiderverse, 3 arcane, 0 realistic, 4 clay
uniform vec2 frame;          // width, height
const vec3 KEY = vec3(-0.72, 0.5, 0.48);
vec3 lightDir() { return normalize(style == 3 ? vec3(-0.85, 0.42, 0.38) : KEY); }
// fframes pixel coordinates (y down) of a fragment.
vec2 px(ivec2 p) { return vec2(float(p.x), frame.y - 1.0 - float(p.y)); }
vec2 meters(ivec2 p) { return (px(p) - anchor) / max(ppm, 1.0); }
float alphaAt(ivec2 p) { return texelFetch(normals, p, 0).a; }
vec3 nAt(ivec2 p) { vec3 n = texelFetch(blurredN, p, 0).xyz; return length(n) > 1e-6 ? normalize(n) : vec3(0.0, 0.0, 1.0); }
vec3 dataAt(ivec2 p) { vec4 d = texelFetch(data, p, 0); return d.a > 1e-4 ? d.rgb / d.a : vec3(0.0); }
vec2 unpackR(float r) { float nail = ss(0.5, 0.6, r); return vec2(nail, min(r / 0.45, 1.0) * (1.0 - nail)); }
float cover(float d, float r) { return r <= 0.0 ? 0.0 : 1.0 - ss(r - 0.75, r + 0.75, d); }
`;

/** Ink coverage per pixel (shade.rs `lines`). */
export const LINES_FS = /* glsl */ `
${COMMON}
${STYLED_INPUTS}
in vec2 vUv;
void main() {
  ivec2 p = ivec2(gl_FragCoord.xy);
  vec2 d = texelFetch(dist, p, 0).rg;
  float r = outline * 0.5 * scale;
  float a = alphaAt(p);
  vec3 n = nAt(p);
  vec3 dd = dataAt(p);
  vec2 nc = unpackR(clamp(dd.r, 0.0, 1.0));
  float inkC = style == 1 ? 0.7 : style == 2 ? 0.85 : style == 3 ? 0.75 : 0.0;
  float crease = ss(0.15, 0.6, nc.y) * ss(0.15, 0.45, n.z) * (1.0 - nc.x);
  float line;
  if (style == 2) {
    vec2 q = meters(p);
    float wobble = noise2(q.x * 90.0 + 3.1, q.y * 90.0) * 0.8 + 0.6;
    float shade = a > 0.0 ? clamp(1.0 - dot(n, lightDir()), 0.0, 2.0) * 0.3 : 0.3;
    float rr = r * wobble * (0.85 + shade);
    line = max(max(cover(d.r, rr), cover(d.g, rr * 0.55)), crease * inkC);
  } else if (style == 3) {
    vec2 q = meters(p);
    float wobble = noise2(q.x * 140.0, q.y * 140.0 + 7.0) * 0.7 + 0.5;
    line = max(max(cover(d.r, r * wobble), cover(d.g, r * 0.8 * wobble) * 0.85), crease * inkC);
  } else if (style == 1) {
    line = max(max(cover(d.r, r), cover(d.g, r * 0.6)), crease * inkC);
  } else {
    line = cover(d.r, r); // lit looks: the silhouette only
  }
  gl_FragColor = vec4(line, 0.0, 0.0, 1.0);
}
`;

/** Toon, spiderverse and arcane fills, ink and plates (shade.rs stylized): premultiplied linear. */
export const STYLED_FS = /* glsl */ `
${COMMON}
${STYLED_INPUTS}
uniform sampler2D lines;
uniform vec3 skin, nailC, palmRatio, ink;
in vec2 vUv;
struct Pal { vec3 deep, shadow, mid, light; };
Pal toon(vec3 c) { return Pal(c * vec3(0.42, 0.26, 0.25), c * vec3(0.62, 0.42, 0.40), c, min(c * 1.22 + vec3(0.05, 0.035, 0.02), vec3(1.0))); }
Pal arcanePal(vec3 c) {
  float l = lum3(c);
  vec3 cool = vec3(0.30, 0.30, 0.46) * l * 1.6;
  return Pal(mix(c * vec3(0.30, 0.22, 0.26), cool * 0.45, 0.35), mix(c * vec3(0.58, 0.44, 0.46), cool * 0.75, 0.3), c * vec3(0.8, 0.7, 0.66),
    min(c * vec3(1.25, 1.08, 0.9) + vec3(0.05, 0.025, 0.0), vec3(1.0)));
}
float lineAt(vec2 q) { // fframes pixel coords
  ivec2 p = ivec2(int(floor(q.x + 0.5)), int(frame.y - 1.0 - floor(q.y + 0.5)));
  if (p.x < 0 || p.y < 0 || float(p.x) >= frame.x || float(p.y) >= frame.y) return 0.0;
  return texelFetch(lines, p, 0).r;
}
float roundf(float x) { return x < 0.0 ? -floor(-x + 0.5) : floor(x + 0.5); }
void main() {
  ivec2 p = ivec2(gl_FragCoord.xy);
  float a = alphaAt(p);
  float line = texelFetch(lines, p, 0).r;
  vec4 outc = vec4(0.0);
  vec3 l = lightDir();
  if (a > 0.0) {
    vec3 n = nAt(p);
    float nl = dot(n, l);
    vec3 dd = dataAt(p);
    float m = unpackR(clamp(dd.r, 0.0, 1.0)).x;
    float detail = clamp(dd.g, 0.0, 1.0);
    vec3 col;
    if (style == 3) {
      Pal ps = arcanePal(skin), pn = arcanePal(nailC);
      float stroke = detail - 0.5;
      nl += stroke * 0.14 * texture_;
      float soft = 0.17;
      float t0 = ss(-0.5 - soft, -0.5 + soft, nl), t1 = ss(-0.1 - soft, -0.1 + soft, nl), t2 = ss(0.42 - soft, 0.42 + soft, nl);
      vec3 bs = mix(mix(mix(ps.deep, ps.shadow, t0), ps.mid, t1), ps.light, t2);
      vec3 bn = mix(mix(mix(pn.deep, pn.shadow, t0), pn.mid, t1), pn.light, t2);
      col = mix(bs, bn, m);
      col *= max(vec3(1.0) + vec3(0.16, 0.06, -0.1) * stroke * texture_, vec3(0.2));
      float rim = ss(0.45, 0.95, 1.0 - max(n.z, 0.0)) * (1.0 - t1);
      col += vec3(0.22, 0.09, 0.03) * rim * lum3(skin) * 1.4;
    } else {
      Pal ps = toon(skin), pn = toon(nailC);
      float aa = 0.025;
      float t1 = ss(0.18 - aa, 0.18 + aa, nl), t2 = ss(0.72 - aa, 0.72 + aa, nl);
      vec3 bs = mix(mix(ps.shadow, ps.mid, t1), ps.light, t2);
      vec3 bn = mix(mix(pn.shadow, pn.mid, t1), pn.light, t2);
      col = mix(min(bs * mix(vec3(1.0), palmRatio, detail), vec3(1.0)), bn, m);
      if (halftone > 0.0) {
        vec2 q = (px(p) - anchor) / halftone;
        float u = (q.x + q.y) * 0.70710678, v = (q.y - q.x) * 0.70710678;
        float d = length(vec2(u - roundf(u), v - roundf(v))) * halftone;
        float dark = max(1.0 - ss(-0.2, 0.45, nl), (1.0 - t1) * 0.55);
        float radius = halftone * 0.62 * sqrt(dark);
        float dotc = cover(d, radius) * 0.9;
        col = mix(col, mix(ps.deep, pn.deep, m), dotc);
      }
    }
    outc = vec4(col * a, a);
  }
  float la = max(line, 0.0);
  outc.rgb = outc.rgb * (1.0 - la) + ink * la;
  outc.a = outc.a * (1.0 - la) + la;
  if (misreg > 0.0) {
    vec2 q = px(p);
    float dx = misreg, dy = misreg * 0.35;
    float gm = lineAt(vec2(q.x - dx, q.y - dy)) * 0.85;
    float gc = lineAt(vec2(q.x + dx * 0.8, q.y + dy * 1.4)) * 0.8;
    float ga = max(gm, gc);
    if (ga > 0.0) {
      vec3 magenta = vec3(0.85, 0.02, 0.32), cyan = vec3(0.0, 0.55, 0.85);
      vec3 gcol = (magenta * gm + cyan * gc * (1.0 - gm)) / max(gm + gc * (1.0 - gm), 1e-4);
      float behind = 1.0 - outc.a;
      outc.rgb += gcol * ga * behind;
      outc.a += ga * behind;
    }
  }
  gl_FragColor = outc;
}
`;

/** The colour pass tone mapped (three's / Khronos PBR Neutral) to (c * a, a). */
export const TONEMAP_FS = /* glsl */ `
uniform sampler2D src;
uniform float exposure;
in vec2 vUv;
vec3 neutral(vec3 color) {
  const float StartCompression = 0.8 - 0.04;
  const float Desaturation = 0.15;
  color *= exposure;
  float x = min(color.r, min(color.g, color.b));
  float offset = x < 0.08 ? x - 6.25 * x * x : 0.04;
  color -= offset;
  float peak = max(color.r, max(color.g, color.b));
  if (peak < StartCompression) return color;
  float d = 1.0 - StartCompression;
  float newPeak = 1.0 - d * d / (peak + d - StartCompression);
  color *= newPeak / peak;
  float g = 1.0 - 1.0 / (Desaturation * (peak - newPeak) + 1.0);
  return mix(color, vec3(newPeak), g);
}
void main() {
  vec4 c = texelFetch(src, ivec2(gl_FragCoord.xy), 0);
  float a = clamp(c.a, 0.0, 1.0);
  vec3 s = a > 1e-4 ? clamp(neutral(c.rgb / c.a), 0.0, 1.0) : vec3(0.0);
  gl_FragColor = vec4(s * a, a);
}
`;

/** Realistic (subsurface mix and warm shadows) and clay, with optional silhouette ink. */
export const LIT_FS = /* glsl */ `
${COMMON}
uniform sampler2D src;   // (c * a, a)
uniform sampler2D sssR, sssG, sssB; // src blurred per channel's reach
uniform sampler2D lines;
uniform float mixAmount;
uniform int realistic;
in vec2 vUv;
void main() {
  ivec2 p = ivec2(gl_FragCoord.xy);
  vec4 s = texelFetch(src, p, 0);
  float a = s.a;
  vec3 c = a > 1e-4 ? s.rgb / a : vec3(0.0);
  if (realistic == 1 && a > 0.0) {
    vec4 r = texelFetch(sssR, p, 0), g = texelFetch(sssG, p, 0), b = texelFetch(sssB, p, 0);
    vec3 bl = vec3(r.a > 1e-3 ? r.r / r.a : 0.0, g.a > 1e-3 ? g.g / g.a : 0.0, b.a > 1e-3 ? b.b / b.a : 0.0);
    c = mix(c, bl, mixAmount);
    float dark = 1.0 - ss(0.0, 0.2, lum3(c));
    c *= mix(vec3(1.0), vec3(1.15, 0.95, 0.88), dark * 0.45);
  }
  vec3 col = a > 0.0 ? c * a : vec3(0.0);
  float la = texelFetch(lines, p, 0).r;
  gl_FragColor = vec4(col * (1.0 - la) + vec3(0.01) * la, a * (1.0 - la) + la);
}
`;

/** Premultiplied linear (`scale` x the layer, summed sub-frames) to premultiplied sRGB, over an
 * optional backdrop. */
export const ENCODE_FS = /* glsl */ `
uniform sampler2D src;
uniform float weight;
uniform vec4 backdrop; // straight sRGB, a = 0 for none
in vec2 vUv;
float enc(float c) { c = clamp(c, 0.0, 1.0); return c <= 0.0031308 ? c * 12.92 : 1.055 * pow(c, 1.0 / 2.4) - 0.055; }
void main() {
  vec4 c = texelFetch(src, ivec2(gl_FragCoord.xy), 0) * weight;
  float a = clamp(c.a, 0.0, 1.0);
  vec3 s = a > 1e-5 ? vec3(enc(c.r / c.a), enc(c.g / c.a), enc(c.b / c.a)) : vec3(0.0);
  vec4 o = vec4(s * a, a);
  if (backdrop.a > 0.0) o = vec4(o.rgb + backdrop.rgb * backdrop.a * (1.0 - a), a + backdrop.a * (1.0 - a));
  gl_FragColor = o;
}
`;

/** The data pass: the data map unlit (R nail / crease, G palm or tint), B the distance from the
 * camera. Skinned and morphed like any three.js mesh. */
export const DATA_PASS_VS = /* glsl */ `
#include <common>
#include <morphtarget_pars_vertex>
#include <skinning_pars_vertex>
out vec2 vUv;
out float vDist;
void main() {
  vUv = uv;
  #include <skinbase_vertex>
  #include <begin_vertex>
  #include <morphinstance_vertex>
  #include <morphtarget_vertex>
  #include <skinning_vertex>
  #include <project_vertex>
  vDist = length(mvPosition.xyz);
}
`;

export const DATA_PASS_FS = /* glsl */ `
uniform sampler2D map;
uniform int hasMap;
in vec2 vUv;
in float vDist;
void main() {
  vec2 d = hasMap == 1 ? texture(map, vUv).rg : vec2(0.0);
  gl_FragColor = vec4(d, vDist, 1.0);
}
`;
