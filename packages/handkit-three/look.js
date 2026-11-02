// @handkit/three looks: the five Handkit looks (realistic, toon, spiderverse, arcane, clay) in
// three.js, ported from fframes (crates/handkit-fframes/src/look) so both engines draw the same
// hands. It needs the hand .glb from `handkit mesh` (its skin maps and masks) and WebGL2.
//
//   import { init, look as lookOf } from '@handkit/three/core.js';
//   import { HandkitLook } from '@handkit/three/look.js';
//   await init();
//   const look = new HandkitLook(renderer, { style: 'toon', skin: 'tan' });
//   await look.dress(gltf);                 // each hand, once (bakes its look textures)
//   await look.dress(gltfAna, clipLooks['ana.right']);   // a hand in a person's own colours
//   look.render(scene, camera);             // instead of renderer.render(scene, camera)
//
// Every look renders the scene through its own passes and post-process into the renderer's
// current target (the canvas by default), transparent where nothing is drawn, as
// premultiplied sRGB. Textures that stay on the skin (tone colour map, crease lines, arcane
// brush strokes, clay lumps and fingerprints) are baked once per hand on the GPU in the
// hand's UV atlas, with the same functions fframes uses.
import * as THREE from 'three';
import { look as resolveLookJson } from './core.js';
import * as G from './look-glsl.js';

export const STYLES = ['realistic', 'toon', 'spiderverse', 'arcane', 'clay'];
const STYLE_ID = { realistic: 0, toon: 1, spiderverse: 2, arcane: 3, clay: 4 };

// ---- colour (handkit_mesh::skin) -------------------------------------------------------
const toLinear = (v) => { const c = v / 255; return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4); };
const toSrgb = (c) => { c = Math.min(Math.max(c, 0), 1); return c <= 0.0031308 ? c * 12.92 : 1.055 * Math.pow(c, 1 / 2.4) - 0.055; };
const linear = (rgb) => rgb.map(toLinear);
const lum = (c) => 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
const mix = (a, b, t) => a.map((v, i) => v + (b[i] - v) * t);
const mul = (a, b) => a.map((v, i) => v * b[i]);
const ss = (e0, e1, x) => { const t = Math.min(Math.max((x - e0) / (e1 - e0), 0), 1); return t * t * (3 - 2 * t); };
const darkness = (t) => ss(0.3, 0.04, lum(t));
const BAKED_TONE = [0xd8, 0xa6, 0x8a];
function nailColor(t) { return mix(mul(t, [1.02, 0.9, 0.92]), linear([0xe0, 0xb8, 0xaa]), 0.42 + 0.18 * darkness(t)); }
function toneBase(tone, nail, m) {
  const dark = darkness(tone);
  const back = mul(tone, [0.92, 0.95, 0.94]);
  const pink = mul(tone, [0.95, 0.95, 1.07]);
  const pale = mix(tone, linear([0xd6, 0x9e, 0x8a]), 0.55);
  const palm = mix(pink, pale, 0.8 * dark);
  let skin = mix(back, palm, m[3]);
  skin = mix(skin, [lum(skin), lum(skin), lum(skin)], 0.2 * dark);
  skin = mix(skin, mul(skin, [1, 0.62, 0.68]), 0.35 * m[0] * (1 - 0.35 * dark));
  skin = mul(skin, mix([1, 1, 1], [0.78, 0.68, 0.66], m[2]));
  return mix(skin, nail, m[1]);
}
const vec3 = (a) => new THREE.Vector3(a[0], a[1], a[2]);
/** The most skin tones (people's colours) one stylized frame tells apart. */
const MAX_TONES = G.MAX_TONES;

/** A look object (or style name) checked by the Handkit core, every default filled in. Needs
 * `init()` from core.js; an already resolved look passes through. */
export function resolveLook(look) {
  if (look && typeof look === 'object' && 'nail_given' in look) return look;
  return JSON.parse(resolveLookJson(JSON.stringify(look ?? null)));
}

// ---- the studio environment (fframes_three_d Environment::studio) ------------------------
const STUDIO = {
  sky: [0.05, 0.055, 0.065], horizon: [0.12, 0.12, 0.13], ground: [0.02, 0.02, 0.022],
  boxes: [
    [[-0.6, 0.75, 0.6], 50, 30, [3.2, 3.2, 3.3]],
    [[1.0, 0.2, 0.15], 14, 70, [2.4, 2.45, 2.6]],
    [[-0.3, 0.35, -1.0], 70, 8, [1.8, 1.9, 2.1]],
    [[0, 1, 0], 40, 40, [0.9, 0.9, 0.95]],
  ].map(([d, w, h, c]) => {
    const dir = new THREE.Vector3(...d).normalize();
    let u = new THREE.Vector3().crossVectors(dir, new THREE.Vector3(0, 1, 0));
    u = u.lengthSq() > 1e-12 ? u.normalize() : new THREE.Vector3(1, 0, 0);
    return { dir, u, v: new THREE.Vector3().crossVectors(u, dir), half: [(w * Math.PI) / 360, (h * Math.PI) / 360], color: c };
  }),
};

function studioRadiance(d) {
  const soft = 0.015, width = 0.08;
  const y = d.y;
  const c = y >= 0 ? mix(STUDIO.horizon, STUDIO.sky, ss(0, width + 0.35, y)) : mix(STUDIO.horizon, STUDIO.ground, ss(0, width, -y));
  for (const b of STUDIO.boxes) {
    const dd = d.dot(b.dir);
    const [hx, hy] = b.half;
    const a = hx + soft, bb = hy + soft;
    if (dd <= -0.2 || (Math.max(a, bb) < Math.PI / 2 && dd <= Math.max(Math.cos(Math.sqrt(a * a + bb * bb)), 0))) continue;
    const inside = (angle, half) => 1 - ss(half - soft * 0.5, half + soft, angle);
    const mask = inside(Math.abs(Math.atan2(d.dot(b.u), dd)), hx) * inside(Math.abs(Math.atan2(d.dot(b.v), dd)), hy);
    if (mask > 0) {
      const energy = (hx * hy) / ((hx + soft * 0.75) * (hy + soft * 0.75));
      for (let k = 0; k < 3; k++) c[k] += b.color[k] * mask * energy;
    }
  }
  return c;
}

/** The studio as an equirectangular panorama, turned `rotation` radians about Y. */
function studioPanorama(rotation) {
  const W = 512, H = 256;
  const data = new Float32Array(W * H * 4);
  const d = new THREE.Vector3();
  const [cr, sr] = [Math.cos(-rotation), Math.sin(-rotation)];
  for (let y = 0; y < H; y++) {
    const lat = ((y + 0.5) / H - 0.5) * Math.PI;
    for (let x = 0; x < W; x++) {
      const phi = ((x + 0.5) / W - 0.5) * 2 * Math.PI;
      const wx = Math.cos(lat) * Math.cos(phi), wy = Math.sin(lat), wz = Math.cos(lat) * Math.sin(phi);
      // fframes samples the unrotated studio at rotate_y(dir, -rotation).
      d.set(wx * cr + wz * sr, wy, -wx * sr + wz * cr);
      const c = studioRadiance(d);
      data.set([c[0], c[1], c[2], 1], (y * W + x) * 4);
    }
  }
  const t = new THREE.DataTexture(data, W, H, THREE.RGBAFormat, THREE.FloatType);
  t.mapping = THREE.EquirectangularReflectionMapping;
  t.colorSpace = THREE.LinearSRGBColorSpace;
  t.magFilter = t.minFilter = THREE.LinearFilter;
  t.needsUpdate = true;
  return t;
}

// ---- small GPU helpers -----------------------------------------------------------------
function target(w, h, opts = {}) {
  const rt = new THREE.WebGLRenderTarget(w, h, {
    type: opts.type ?? THREE.HalfFloatType,
    format: THREE.RGBAFormat,
    colorSpace: THREE.NoColorSpace,
    depthBuffer: !!opts.depth,
    samples: opts.samples ?? 0,
    minFilter: opts.mipmaps ? THREE.LinearMipmapLinearFilter : opts.nearest ? THREE.NearestFilter : THREE.LinearFilter,
    magFilter: opts.nearest ? THREE.NearestFilter : THREE.LinearFilter,
    generateMipmaps: !!opts.mipmaps,
  });
  rt.texture.wrapS = rt.texture.wrapT = THREE.ClampToEdgeWrapping;
  return rt;
}

class Quad {
  constructor() {
    this.camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0, 1);
    this.mesh = new THREE.Mesh(new THREE.PlaneGeometry(2, 2));
    this.mesh.frustumCulled = false;
    this.scene = new THREE.Scene();
    this.scene.add(this.mesh);
    this.materials = new Map();
  }
  material(fs, uniforms = {}) {
    let m = this.materials.get(fs);
    if (!m) {
      m = new THREE.ShaderMaterial({ vertexShader: G.QUAD_VS, fragmentShader: fs, uniforms: {}, depthTest: false, depthWrite: false, blending: THREE.NoBlending });
      this.materials.set(fs, m);
    }
    for (const [k, v] of Object.entries(uniforms)) {
      if (m.uniforms[k]) m.uniforms[k].value = v; else m.uniforms[k] = { value: v };
    }
    return m;
  }
  /** Runs fragment shader `fs` into `rt` (null: the renderer's own target). */
  run(renderer, fs, uniforms, rt, blending = THREE.NoBlending) {
    const m = this.material(fs, uniforms);
    m.blending = blending;
    if (blending === THREE.CustomBlending) {
      m.blendSrc = THREE.OneFactor; m.blendDst = THREE.OneFactor; m.blendEquation = THREE.AddEquation;
      m.blendSrcAlpha = THREE.OneFactor; m.blendDstAlpha = THREE.OneFactor;
    }
    this.mesh.material = m;
    renderer.setRenderTarget(rt);
    renderer.render(this.scene, this.camera);
  }
  dispose() { this.mesh.geometry.dispose(); this.materials.forEach((m) => m.dispose()); }
}

const tex = (rt) => rt.texture;

// ---- dressing a hand ---------------------------------------------------------------------
function skinnedMeshOf(root) {
  let found = null;
  root.traverse((o) => { if (!found && o.isSkinnedMesh) found = o; });
  return found;
}

/** The glb image named `name` (Handkit writes skin_color, skin_masks, skin_orm, ...). */
async function glbImage(gltf, name, colorSpace) {
  const parser = gltf.parser;
  const images = parser?.json?.images ?? [];
  const index = images.findIndex((im) => im.name === name);
  if (index < 0) return null;
  const t = await parser.loadImageSource(index, parser.textureLoader);
  const out = t.clone();
  out.flipY = false;
  out.colorSpace = colorSpace;
  out.wrapS = out.wrapT = THREE.ClampToEdgeWrapping;
  out.minFilter = THREE.LinearFilter;
  out.magFilter = THREE.LinearFilter;
  out.generateMipmaps = false;
  out.needsUpdate = true;
  return out;
}

/** The hand's triangles laid out for drawing into its UV atlas: each corner carries the bind
 * pose point and normal, its barycentric corner, and all three corners' skin weights. */
function bakeGeometry(geometry) {
  const pos = geometry.attributes.position, nor = geometry.attributes.normal, uv = geometry.attributes.uv;
  const ji = geometry.attributes.skinIndex, wi = geometry.attributes.skinWeight;
  const index = geometry.index ? geometry.index.array : Array.from({ length: pos.count }, (_, i) => i);
  const n = index.length;
  const arr = (k) => new Float32Array(n * k);
  const P = arr(3), BP = arr(3), BN = arr(3), B = arr(3), J = [arr(4), arr(4), arr(4)], W = [arr(4), arr(4), arr(4)];
  for (let t = 0; t < n; t += 3) {
    const c = [index[t], index[t + 1], index[t + 2]];
    for (let k = 0; k < 3; k++) {
      const o = t + k, v = c[k];
      P.set([uv.getX(v), uv.getY(v), 0], o * 3);
      BP.set([pos.getX(v), pos.getY(v), pos.getZ(v)], o * 3);
      BN.set([nor.getX(v), nor.getY(v), nor.getZ(v)], o * 3);
      B[o * 3 + k] = 1;
      for (let q = 0; q < 3; q++) {
        const w = c[q];
        J[q].set(ji ? [ji.getX(w), ji.getY(w), ji.getZ(w), ji.getW(w)] : [0, 0, 0, 0], o * 4);
        W[q].set(wi ? [wi.getX(w), wi.getY(w), wi.getZ(w), wi.getW(w)] : [1, 0, 0, 0], o * 4);
      }
    }
  }
  const g = new THREE.BufferGeometry();
  g.setAttribute('position', new THREE.BufferAttribute(P, 3));
  g.setAttribute('bp', new THREE.BufferAttribute(BP, 3));
  g.setAttribute('bn', new THREE.BufferAttribute(BN, 3));
  g.setAttribute('bary', new THREE.BufferAttribute(B, 3));
  ['jA', 'jB', 'jC'].forEach((k, q) => g.setAttribute(k, new THREE.BufferAttribute(J[q], 4)));
  ['wA', 'wB', 'wC'].forEach((k, q) => g.setAttribute(k, new THREE.BufferAttribute(W[q], 4)));
  return g;
}

/** One dressed hand: its mesh, loaded maps and the look textures baked so far. */
class Dressed {
  constructor(mesh, maps, colors) {
    this.mesh = mesh;
    this.colors = colors; // a resolved look whose skin and nail this hand wears, or null
    this.loaded = mesh.material;
    this.maps = maps; // { color, masks, orm } or null
    this.baked = new Map();
    this.bake = null; // UV-space geometry, built on first bake
    this.materials = {};
  }
}

// ---- the look ---------------------------------------------------------------------------
export class HandkitLook {
  /**
   * @param renderer a THREE.WebGLRenderer (WebGL2)
   * @param look a look (`{ style, skin, ... }`, a style name, or a resolved look)
   */
  constructor(renderer, look = 'realistic') {
    this.renderer = renderer;
    this.quad = new Quad();
    this.hands = [];
    this.targets = null;
    this.envs = new Map();
    this.pmrem = new THREE.PMREMGenerator(renderer);
    this.backdrop = null;
    this.lights = new THREE.Group();
    this.setLook(look);
  }

  setLook(look) {
    this.look = resolveLook(look);
    this.style = STYLE_ID[this.look.style];
    for (const h of this.hands) h.materials = {};
  }

  /** A solid colour behind the hands (`'#767879'`, a THREE.Color or null for transparent). */
  setBackdrop(color) {
    this.backdrop = color == null ? null : new THREE.Color(color);
  }

  /**
   * Prepares a loaded hand (the GLTFLoader result, or its scene for a hand without maps).
   * `colors`, optional: a look (e.g. a person's, `Clip.looks()[hand]`) whose skin and nail this
   * hand wears instead of the look's; its style is ignored. Every style shows it.
   */
  async dress(gltf, colors = null) {
    const root = gltf.scene ?? gltf;
    const mesh = skinnedMeshOf(root);
    if (!mesh) throw new Error('HandkitLook.dress: no skinned mesh in this glTF (use a hand from `handkit mesh`)');
    mesh.frustumCulled = false;
    let maps = null;
    if (gltf.parser) {
      const [color, masks, orm] = await Promise.all([
        glbImage(gltf, 'skin_color', THREE.SRGBColorSpace),
        glbImage(gltf, 'skin_masks', THREE.NoColorSpace),
        glbImage(gltf, 'skin_orm', THREE.NoColorSpace),
      ]);
      if (color && masks && orm && mesh.geometry.attributes.uv) maps = { color, masks, orm };
    }
    const d = new Dressed(mesh, maps, colors ? resolveLook(colors) : null);
    this.hands.push(d);
    return d;
  }

  // ---- textures baked in the atlas ----
  bakeMaterial(d) {
    if (!d.bakeMat) {
      const bones = Math.max(d.mesh.skeleton?.boneInverses.length ?? 1, 1);
      d.bakeMat = new THREE.ShaderMaterial({
        vertexShader: G.BAKE_VS,
        fragmentShader: G.BAKE_FS(bones),
        uniforms: {
          mode: { value: 0 },
          size: { value: 1024 },
          ibm: { value: d.mesh.skeleton ? d.mesh.skeleton.boneInverses : [new THREE.Matrix4()] },
        },
        side: THREE.DoubleSide,
        depthTest: false,
        depthWrite: false,
        blending: THREE.NoBlending,
      });
      d.bake = new THREE.Mesh(bakeGeometry(d.mesh.geometry), d.bakeMat);
      d.bake.frustumCulled = false;
      d.bakeScene = new THREE.Scene();
      d.bakeScene.add(d.bake);
    }
    return d.bakeMat;
  }

  /** Draws the mesh into an atlas-sized target in `mode`, then grows the charts by 2 texels. */
  atlasPass(d, mode, size, opts = {}) {
    const r = this.renderer;
    const m = this.bakeMaterial(d);
    m.uniforms.mode.value = mode;
    m.uniforms.size.value = size;
    const a = target(size, size, { type: opts.type ?? THREE.HalfFloatType, nearest: true });
    const b = target(size, size, { type: opts.type ?? THREE.HalfFloatType, nearest: true });
    r.setRenderTarget(a);
    r.setClearColor(0x000000, 0);
    r.clear();
    r.render(d.bakeScene, this.quad.camera);
    for (let i = 0; i < 2; i++) {
      this.quad.run(r, G.DILATE_FS, { src: tex(i ? b : a) }, i ? a : b);
    }
    b.dispose();
    return a;
  }

  /** A baked texture for hand `d`: 'tone', 'data', 'arcane-data', 'arcane-normal', 'clay-normal'. */
  baked(d, kind) {
    const key = kind === 'tone' ? `tone:${this.colorsOf(d).skin}:${this.realisticNail(d)}` : kind;
    if (d.baked.has(key)) return d.baked.get(key);
    if (!d.maps && kind !== 'clay-normal' && kind !== 'arcane-normal') return null;
    const r = this.renderer;
    const prev = { target: r.getRenderTarget(), clear: r.getClearColor(new THREE.Color()), alpha: r.getClearAlpha() };
    let out;
    if (kind === 'tone') {
      const size = d.maps.color.image.width;
      out = target(size, size, { mipmaps: true });
      out.texture.anisotropy = r.capabilities.getMaxAnisotropy();
      this.quad.run(r, G.TONE_MAP_FS, {
        color: d.maps.color, masks: d.maps.masks, orm: d.maps.orm,
        tone: vec3(linear(this.colorsOf(d).skin)), nail: vec3(this.realisticNailLinear(d)), bakedTone: vec3(linear(BAKED_TONE)),
      }, out);
      out.texture.colorSpace = THREE.LinearSRGBColorSpace;
    } else if (kind === 'data' || kind === 'arcane-data') {
      const size = d.maps.masks.image.width;
      const cover = this.atlasPass(d, 0, size);
      const cav = target(size, size);
      this.quad.run(r, G.CAVITY_FS, { masks: d.maps.masks, cover: tex(cover) }, cav);
      const tmp = target(size, size);
      const sigma = (1.6 * size) / 1024;
      this.quad.run(r, G.BLUR_FS, { src: tex(cav), dir: new THREE.Vector2(1, 0), sigma, packed: 0 }, tmp);
      this.quad.run(r, G.BLUR_FS, { src: tex(tmp), dir: new THREE.Vector2(0, 1), sigma, packed: 0 }, cav);
      const tint = kind === 'arcane-data' ? this.atlasPass(d, 1, 1024) : null;
      out = target(size, size, { type: THREE.UnsignedByteType });
      this.quad.run(r, G.DATA_FS, { masks: d.maps.masks, orm: d.maps.orm, cover: tex(cover), blurred: tex(cav), tint: tint ? tex(tint) : tex(cover), arcane: tint ? 1 : 0 }, out);
      [cover, cav, tmp, tint].forEach((t) => t?.dispose());
    } else {
      const [mode, size] = kind === 'arcane-normal' ? [2, 1024] : [3, 2048];
      const raw = this.atlasPass(d, mode, size, { type: THREE.UnsignedByteType });
      // Copy into a mipmapped target for sampling (empty texels stay flat).
      out = target(size, size, { type: THREE.UnsignedByteType, mipmaps: true });
      this.quad.run(r, `uniform sampler2D src; in vec2 vUv; void main() { vec4 c = texelFetch(src, ivec2(gl_FragCoord.xy), 0); gl_FragColor = c.a > 0.5 ? vec4(c.rgb, 1.0) : vec4(0.5, 0.5, 1.0, 1.0); }`, { src: tex(raw) }, out);
      raw.dispose();
    }
    r.setRenderTarget(prev.target);
    r.setClearColor(prev.clear, prev.alpha);
    d.baked.set(key, out.texture);
    return out.texture;
  }

  /** The skin and nail a hand wears: its own colours, else the look's. */
  colorsOf(d) {
    const c = d?.colors ?? this.look;
    return { skin: c.skin, nail: c.nail, nail_given: c.nail_given };
  }

  realisticNail(d) {
    const c = this.colorsOf(d);
    if (c.nail_given) return c.nail;
    return nailColor(linear(c.skin)).map((v) => Math.round(toSrgb(v) * 255));
  }
  realisticNailLinear(d) { return linear(this.realisticNail(d)); }

  /** The distinct colours of the dressed hands in `scene`, and each hand's index among them. */
  tones(scene) {
    const list = [];
    const index = new Map();
    for (const d of this.hands) {
      if (!this.inScene(scene, d.mesh)) continue;
      const c = this.colorsOf(d);
      let i = list.findIndex((t) => t.skin.join() === c.skin.join() && t.nail.join() === c.nail.join());
      if (i < 0) { i = list.length; list.push(c); }
      index.set(d, i);
    }
    if (!list.length) list.push(this.colorsOf(null));
    return { list: list.slice(0, MAX_TONES), index };
  }

  // ---- materials ----
  normalScale(d, s) {
    const base = d.loaded.normalScale ?? new THREE.Vector2(1, 1);
    return new THREE.Vector2(Math.sign(base.x || 1) * s, Math.sign(base.y || 1) * s);
  }

  /** Vertex colours from the nail mask (TEXCOORD_1.x): `body` mixed toward `nail`. */
  vertexColors(d, body, nail, amount) {
    const g = d.mesh.geometry;
    const uv1 = g.attributes.uv1;
    const n = g.attributes.position.count;
    const c = new Float32Array(n * 3);
    for (let i = 0; i < n; i++) c.set(mix(body, nail, (uv1 ? uv1.getX(i) : 0) * amount), i * 3);
    g.setAttribute('color', new THREE.BufferAttribute(c, 3));
  }

  materials(d) {
    if (d.materials.color) return d.materials;
    const L = this.look;
    const own = this.colorsOf(d);
    const skin = linear(own.skin);
    const nail = linear(own.nail);
    const out = {};
    if (this.style === 0) {
      const map = this.baked(d, 'tone');
      if (map) {
        // Skin detail seen at a slant stays sharp (fframes filters by the pixel footprint).
        const aniso = this.renderer.capabilities.getMaxAnisotropy();
        for (const t of [d.loaded.normalMap, d.loaded.roughnessMap]) if (t && t.anisotropy !== aniso) { t.anisotropy = aniso; t.needsUpdate = true; }
        out.color = new THREE.MeshPhysicalMaterial({
          map, roughness: 1, metalness: 0, clearcoat: 0.1, clearcoatRoughness: 0.45,
          normalMap: d.loaded.normalMap ?? null, normalScale: d.loaded.normalScale?.clone() ?? new THREE.Vector2(1, 1),
          roughnessMap: d.loaded.roughnessMap ?? null, aoMap: d.loaded.aoMap ?? null, aoMapIntensity: d.loaded.aoMapIntensity ?? 1,
        });
      } else {
        const l = lum(skin);
        const body = mul(mix(skin, [l, l, l], Math.min(Math.max(0.18 + 0.25 * (1 - l / 0.6), 0.18), 0.43)), [1.02, 0.97, 1]);
        this.vertexColors(d, body, this.realisticNailLinear(d), 1);
        out.color = new THREE.MeshPhysicalMaterial({ vertexColors: true, roughness: 0.52, metalness: 0, clearcoat: 0.12, clearcoatRoughness: 0.45 });
      }
    } else if (this.style === 4) {
      this.vertexColors(d, mul(skin, [1.04, 0.93, 0.8]), nail, 0.3);
      out.color = new THREE.MeshStandardMaterial({
        vertexColors: true, roughness: 0.9, metalness: 0, envMapIntensity: 0.7,
        normalMap: this.baked(d, 'clay-normal'), normalScale: this.normalScale(d, L.texture),
      });
    } else {
      const arcane = this.style === 3;
      out.normals = new THREE.MeshNormalMaterial(arcane ? { normalMap: this.baked(d, 'arcane-normal'), normalScale: this.normalScale(d, L.texture) } : {});
      const map = this.baked(d, arcane ? 'arcane-data' : 'data');
      out.data = new THREE.ShaderMaterial({
        vertexShader: G.DATA_PASS_VS, fragmentShader: G.DATA_PASS_FS,
        uniforms: { map: { value: map }, hasMap: { value: map ? 1 : 0 } },
      });
      out.color = out.normals;
      // Which tone a pixel shows, for scenes of several people (the hand's skin, unlit).
      out.tone = new THREE.MeshBasicMaterial({ color: new THREE.Color().setRGB(skin[0], skin[1], skin[2], THREE.LinearSRGBColorSpace) });
    }
    d.materials = out;
    return out;
  }

  // ---- per frame ----
  ensureTargets() {
    const size = this.renderer.getDrawingBufferSize(new THREE.Vector2());
    const [w, h] = [size.x, size.y];
    if (this.targets && this.targets.w === w && this.targets.h === h) return this.targets;
    if (this.targets) Object.values(this.targets).forEach((t) => t.dispose?.());
    const t = (o) => target(w, h, { nearest: true, ...o });
    this.targets = {
      w, h,
      scene: t({ samples: 4, depth: true }), data: t({ samples: 4, depth: true }), tone: t({ samples: 4, depth: true }),
      a: t(), b: t(), c: t(), d: t(), e: t(), f: t(), lines: t(), layer: t(), accum: t(),
    };
    return this.targets;
  }

  environment(rotation) {
    const key = Math.round(rotation * 200);
    let env = this.envs.get(key);
    if (!env) {
      const pano = studioPanorama(key / 200);
      env = this.pmrem.fromEquirectangular(pano).texture;
      pano.dispose();
      if (this.envs.size > 16) { const [k, v] = this.envs.entries().next().value; v.dispose(); this.envs.delete(k); }
      this.envs.set(key, env);
    }
    return env;
  }

  /** The look's lights, placed relative to the camera (Styler::light). */
  placeLights(camera, center, radius) {
    const g = this.lights;
    g.clear();
    const e = camera.matrixWorld.elements;
    const right = new THREE.Vector3(e[0], e[1], e[2]).normalize();
    const up = new THREE.Vector3(e[4], e[5], e[6]).normalize();
    const back = new THREE.Vector3(e[8], e[9], e[10]).normalize();
    const dir = (x, y, z) => right.clone().multiplyScalar(x).addScaledVector(up, y).addScaledVector(back, z).normalize();
    const clay = this.style === 4;
    const hemi = clay ? new THREE.HemisphereLight(0xfffaf4, 0x5a5048, 1.25) : new THREE.HemisphereLight(0xffffff, 0x505055, 1.0);
    g.add(hemi);
    const sunDir = clay ? dir(-0.45, 0.7, 0.65) : dir(-0.5, 0.7, 0.6);
    const sun = new THREE.DirectionalLight(clay ? 0xfff0e2 : 0xfff6ee, clay ? 2.2 : 2.6);
    sun.position.copy(center).addScaledVector(sunDir, radius * 3);
    sun.target.position.copy(center);
    sun.castShadow = true;
    const s = sun.shadow;
    s.mapSize.set(2048, 2048);
    const cam = s.camera;
    cam.left = cam.bottom = -radius; cam.right = cam.top = radius;
    cam.near = radius * 0.5; cam.far = radius * 5.5;
    cam.updateProjectionMatrix();
    const texel = (2 * radius) / 2048;
    s.radius = clay ? 6 : 4;
    s.intensity = clay ? 0.6 : 0.85;
    s.bias = -(4 * texel) / (cam.far - cam.near);
    s.normalBias = 1.5 * texel;
    g.add(sun, sun.target);
    const fill = new THREE.DirectionalLight(clay ? 0xe6ecff : 0xe8f0ff, clay ? 0.5 : 1.2);
    fill.position.copy(center).addScaledVector(clay ? dir(0.8, 0.1, 0.5) : dir(0.8, 0.35, -0.55), radius * 3);
    fill.target.position.copy(center);
    g.add(fill, fill.target);
    g.updateMatrixWorld(true);
    return Math.atan2(back.x, back.z);
  }

  /** Where the hands are: the joints' bounding sphere and the anchor (the first hand bone). */
  handsFrame(scene) {
    const box = new THREE.Box3();
    let anchor = null;
    for (const d of this.hands) {
      if (!this.inScene(scene, d.mesh)) continue;
      for (const b of d.mesh.skeleton?.bones ?? []) box.expandByPoint(b.getWorldPosition(new THREE.Vector3()));
    }
    for (const name of ['hand_R', 'hand_L']) {
      const o = scene.getObjectByName(name);
      if (o) { anchor = o.getWorldPosition(new THREE.Vector3()); break; }
    }
    const sphere = box.isEmpty() ? new THREE.Sphere(new THREE.Vector3(), 0.3) : box.getBoundingSphere(new THREE.Sphere());
    return { center: sphere.center, radius: Math.max(sphere.radius + 0.08, 0.15), anchor: anchor ?? sphere.center };
  }

  inScene(scene, o) {
    for (let p = o; p; p = p.parent) if (p === scene) return true;
    return false;
  }

  /** Swaps every mesh's material for the pass ('color', 'normals', 'data', 'tone'); returns an undo. */
  swap(scene, pass) {
    const saved = [];
    const generic = this.generic ??= {
      normals: new THREE.MeshNormalMaterial(),
      data: new THREE.ShaderMaterial({ vertexShader: G.DATA_PASS_VS, fragmentShader: G.DATA_PASS_FS, uniforms: { map: { value: null }, hasMap: { value: 0 } } }),
      tone: new THREE.MeshBasicMaterial({ color: 0x000000 }),
    };
    scene.traverse((o) => {
      if (!o.isMesh) return;
      const d = this.hands.find((h) => h.mesh === o);
      saved.push([o, o.material, o.visible, o.castShadow, o.receiveShadow]);
      if (d) {
        o.material = this.materials(d)[pass];
        o.castShadow = o.receiveShadow = pass === 'color';
      } else if (pass !== 'color') {
        const m = Array.isArray(o.material) ? o.material[0] : o.material;
        if (m?.transparent || (m?.opacity ?? 1) < 1 || m?.isShadowMaterial) o.visible = false;
        else o.material = generic[pass];
      }
    });
    return () => saved.forEach(([o, m, v, c, r]) => { o.material = m; o.visible = v; o.castShadow = c; o.receiveShadow = r; });
  }

  renderScene(scene, camera, pass, rt) {
    const r = this.renderer;
    const undo = this.swap(scene, pass);
    const bg = scene.background, env = scene.environment, envI = scene.environmentIntensity;
    scene.background = null;
    if (pass !== 'color') scene.environment = null;
    r.setRenderTarget(rt);
    r.setClearColor(0x000000, 0);
    r.clear(true, true, true);
    r.render(scene, camera);
    scene.background = bg; scene.environment = env; scene.environmentIntensity = envI;
    undo();
  }

  /** One moment through the look's passes into `this.targets.layer` (premultiplied linear). */
  layer(scene, camera) {
    const r = this.renderer;
    const T = this.ensureTargets();
    const L = this.look;
    const [W, H] = [T.w, T.h];
    const scale = H / 1080;
    scene.updateMatrixWorld();
    camera.updateMatrixWorld();
    const frame = this.handsFrame(scene);
    // The anchor in pixels (y down) and pixels per meter there.
    const v = frame.anchor.clone().applyMatrix4(camera.matrixWorldInverse);
    const ndc = frame.anchor.clone().project(camera);
    const anchor = new THREE.Vector2((ndc.x * 0.5 + 0.5) * W, (0.5 - ndc.y * 0.5) * H);
    const ppm = camera.isPerspectiveCamera
      ? H / (2 * Math.max(Math.abs(v.z), 1e-4) * Math.tan((camera.fov * Math.PI) / 360))
      : H / ((camera.top - camera.bottom) / camera.zoom);
    const common = { anchor, ppm, scale, outline: L.outline, texture_: L.texture, halftone: 0, misreg: 0, style: this.style, frame: new THREE.Vector2(W, H) };
    const reachFor = (r0) => Math.min(Math.ceil(r0 + 2), 64);
    const edt = (interior, reach) => {
      this.quad.run(r, G.SEEDS_FS, { normals: tex(T.a), blurredN: tex(T.c), data: tex(T.data), interior }, T.d);
      this.quad.run(r, G.EDT_FS, { src: tex(T.d), pass: 0, reach }, T.e);
      this.quad.run(r, G.EDT_FS, { src: tex(T.e), pass: 1, reach }, T.f);
    };

    if (this.style === 1 || this.style === 2 || this.style === 3) {
      this.renderScene(scene, camera, 'normals', T.scene);
      this.renderScene(scene, camera, 'data', T.data);
      // Resolve the normals (a: alpha) and soften them a little inside the alpha.
      this.quad.run(r, 'uniform sampler2D src; in vec2 vUv; void main() { gl_FragColor = texelFetch(src, ivec2(gl_FragCoord.xy), 0); }', { src: tex(T.scene) }, T.a);
      const sigma = 1.6 * scale;
      this.quad.run(r, G.BLUR_FS, { src: tex(T.a), dir: new THREE.Vector2(1, 0), sigma, packed: 1 }, T.b);
      this.quad.run(r, G.BLUR_FS, { src: tex(T.b), dir: new THREE.Vector2(0, 1), sigma, packed: 0 }, T.c);
      const rr = L.outline * 0.5 * scale * (this.style === 2 ? 2.05 : this.style === 3 ? 1.2 : 1);
      edt(1, reachFor(rr));
      const u = { ...common, normals: tex(T.a), blurredN: tex(T.c), data: tex(T.data), dist: tex(T.f) };
      if (this.style === 2) { u.halftone = L.halftone * scale; u.misreg = L.misregister * scale; }
      this.quad.run(r, G.LINES_FS, u, T.lines);
      // Each person's palettes: with more than one, a tone pass says which a pixel shows.
      const tones = this.tones(scene).list;
      if (tones.length > 1) this.renderScene(scene, camera, 'tone', T.tone);
      const pad = (a) => Array.from({ length: MAX_TONES }, (_, i) => vec3(a[Math.min(i, a.length - 1)]));
      const skins = tones.map((c) => linear(c.skin));
      const palms = skins.map((t) => {
        const back = toneBase(t, t, [0, 0, 0, 0]), palm = toneBase(t, t, [0, 0, 0, 1]);
        return palm.map((p, i) => p / Math.max(back[i], 1e-4));
      });
      const inks = skins.map((skin) => (this.style === 3 ? mul(skin, [0.09, 0.045, 0.045]) : this.style === 1 ? [0.02, 0.012, 0.01] : [0.004, 0.004, 0.004]));
      this.quad.run(r, G.STYLED_FS, {
        ...u, lines: tex(T.lines), toneMap: tex(T.tone), toneCount: tones.length,
        skins: pad(skins), nails: pad(tones.map((c) => linear(c.nail))), palms: pad(palms), inks: pad(inks),
      }, T.layer);
    } else {
      // Lit looks: the colour pass under the look's lights and studio, tone mapped.
      const rotation = this.placeLights(camera, frame.center, frame.radius);
      scene.add(this.lights);
      const env = scene.environment, envI = scene.environmentIntensity;
      scene.environment = this.environment(rotation);
      // fframes lights diffuse from the studio analytically; three from its prefiltered map. These
      // gains (measured with tools/looks) make the lit hands as bright as in fframes.
      scene.environmentIntensity = this.style === 4 ? 0.55 * 0.81 : 0.6 * 1.3;
      const sm = { enabled: r.shadowMap.enabled, type: r.shadowMap.type };
      r.shadowMap.enabled = true;
      r.shadowMap.type = THREE.PCFShadowMap;
      try {
        this.renderScene(scene, camera, 'color', T.scene);
      } finally {
        scene.remove(this.lights);
        scene.environment = env; scene.environmentIntensity = envI;
        r.shadowMap.enabled = sm.enabled; r.shadowMap.type = sm.type;
      }
      this.quad.run(r, G.TONEMAP_FS, { src: tex(T.scene), exposure: this.style === 4 ? 1.05 : 1.0 }, T.a);
      const realistic = this.style === 0;
      const blurInto = (sigma, out) => {
        this.quad.run(r, G.BLUR_FS, { src: tex(T.a), dir: new THREE.Vector2(1, 0), sigma, packed: 0 }, T.b);
        this.quad.run(r, G.BLUR_FS, { src: tex(T.b), dir: new THREE.Vector2(0, 1), sigma, packed: 0 }, out);
      };
      if (realistic) {
        // Scattering distances of skin per channel (meters), red the farthest.
        const reach = [0.0024, 0.001, 0.0006].map((m) => Math.min(Math.max(m * ppm, 0), 40 * scale));
        blurInto(reach[0], T.c);
        blurInto(reach[1], T.e);
        blurInto(reach[2], T.f);
      }
      if (L.outline > 0) {
        // Seeds from the colour pass's alpha (T.a holds (c * a, a)); only the silhouette.
        this.quad.run(r, G.SEEDS_FS, { normals: tex(T.a), blurredN: tex(T.a), data: tex(T.a), interior: 0 }, T.d);
        const reach = reachFor(L.outline * 0.5 * scale);
        this.quad.run(r, G.EDT_FS, { src: tex(T.d), pass: 0, reach }, T.b);
        this.quad.run(r, G.EDT_FS, { src: tex(T.b), pass: 1, reach }, T.d);
        this.quad.run(r, G.LINES_FS, { ...common, style: 0, normals: tex(T.a), blurredN: tex(T.a), data: tex(T.a), dist: tex(T.d) }, T.lines);
      } else {
        r.setRenderTarget(T.lines); r.setClearColor(0x000000, 0); r.clear(true, false, false);
      }
      this.quad.run(r, G.LIT_FS, { src: tex(T.a), sssR: tex(T.c), sssG: tex(T.e), sssB: tex(T.f), lines: tex(T.lines), mixAmount: L.subsurface, realistic: realistic ? 1 : 0 }, T.layer);
    }
    return T.layer;
  }

  /**
   * Renders `scene` from `camera` in the look into the renderer's current target.
   * `moment(dt)`, optional, poses the scene `dt` frames from the frame's time: with the look's
   * `motion_blur` above 1 it is called once per sub-frame across the shutter (and with 0 last,
   * leaving the scene at the frame's own pose).
   */
  render(scene, camera, moment = null) {
    const r = this.renderer;
    const out = r.getRenderTarget();
    const autoClear = r.autoClear;
    const clear = { color: r.getClearColor(new THREE.Color()), alpha: r.getClearAlpha() };
    r.autoClear = false;
    const n = moment ? Math.max(1, this.look.motion_blur | 0) : 1;
    const shutter = this.look.shutter / 360;
    let src;
    try {
      if (n === 1) {
        src = this.layer(scene, camera);
      } else {
        const T = this.ensureTargets();
        r.setRenderTarget(T.accum); r.setClearColor(0x000000, 0); r.clear(true, false, false);
        for (let i = 0; i < n; i++) {
          moment(((i + 0.5) / n - 0.5) * shutter);
          const layer = this.layer(scene, camera);
          this.quad.run(r, 'uniform sampler2D src; in vec2 vUv; void main() { gl_FragColor = texelFetch(src, ivec2(gl_FragCoord.xy), 0); }', { src: tex(layer) }, T.accum, THREE.CustomBlending);
        }
        moment(0);
        src = T.accum;
      }
      const bd = this.backdrop;
      r.setRenderTarget(out);
      r.setClearColor(0x000000, 0);
      r.clear(true, true, true);
      this.quad.run(r, G.ENCODE_FS, {
        src: tex(src), weight: 1 / n,
        backdrop: bd ? new THREE.Vector4(toSrgb(bd.r), toSrgb(bd.g), toSrgb(bd.b), 1) : new THREE.Vector4(0, 0, 0, 0),
      }, out);
    } finally {
      r.setRenderTarget(out);
      r.autoClear = autoClear;
      r.setClearColor(clear.color, clear.alpha);
    }
  }

  dispose() {
    if (this.targets) Object.values(this.targets).forEach((t) => t.dispose?.());
    this.envs.forEach((e) => e.dispose());
    this.pmrem.dispose();
    this.quad.dispose();
    for (const d of this.hands) {
      d.baked.forEach((t) => t.dispose());
      Object.values(d.materials).forEach((m) => m.dispose?.());
      d.bake?.geometry.dispose(); d.bakeMat?.dispose();
      d.mesh.material = d.loaded;
    }
  }
}

/**
 * The one-call form: dresses `hands` (GLTFLoader results) for `look` and returns the
 * HandkitLook, its `render()` bound to `scene` and `camera`:
 *   const look = await applyLook(renderer, scene, camera, [gltfR], { style: 'arcane' });
 *   look.draw();   // each frame, instead of renderer.render(scene, camera)
 */
export async function applyLook(renderer, scene, camera, hands, look) {
  const l = new HandkitLook(renderer, look);
  for (const h of hands) await l.dress(h);
  l.draw = (moment) => l.render(scene, camera, moment);
  return l;
}
