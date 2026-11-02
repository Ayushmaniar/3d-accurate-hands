// @handkit/three: poses a Handkit hand mesh (.glb from `handkit mesh`) in three.js.
// The pose comes from the Handkit core (`handkit bones`, or the WASM build's `bones(pose)`):
// { joints: { "index_pip_R": { t: [x, y, z], r: [x, y, z, w] }, ... },
//   morph: { "skin_R": [corrective shape weights] } }.

/** Finds each joint node of a loaded hand once; returns a function that poses it. */
export function handRig(root) {
  const nodes = new Map();
  const meshes = [];
  root.traverse((o) => {
    if (o.name) nodes.set(o.name, o);
    if (o.isSkinnedMesh && o.morphTargetInfluences) meshes.push(o);
  });
  return function apply(bones) {
    for (const mesh of meshes) {
      // The mesh is named "skin_R" (or its parent group is, for multi-primitive meshes).
      const weights = bones.morph?.[mesh.name] ?? bones.morph?.[mesh.parent?.name];
      if (weights) weights.forEach((w, i) => { mesh.morphTargetInfluences[i] = w; });
    }
    for (const [name, b] of Object.entries(bones.joints ?? bones)) {
      const node = nodes.get(name);
      if (!node) continue;
      node.position.set(b.t[0], b.t[1], b.t[2]);
      node.quaternion.set(b.r[0], b.r[1], b.r[2], b.r[3]);
    }
    root.updateMatrixWorld(true);
  };
}

/**
 * The default skin look. A hand with baked skin maps (`handkit mesh`, the default) keeps its own
 * textured material, passed as `loaded` (colour, normal, roughness and occlusion maps); one
 * without (`--textures 0`) gets vertex colors (skin tone, lighter nails) and a soft roughness.
 */
export function skinMaterial(THREE, opts = {}, loaded = null) {
  if (loaded?.map) return loaded;
  return new THREE.MeshStandardMaterial({ vertexColors: true, roughness: opts.roughness ?? 0.55, metalness: 0 });
}
