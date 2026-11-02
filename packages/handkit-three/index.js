// @handkit/three: poses a Handkit hand mesh (.glb from `handkit mesh`) in three.js.
// The pose comes from the Handkit core (`handkit bones`, or the WASM build's `bones(pose)`):
// { joints: { "index_pip_R": { t: [x, y, z], r: [x, y, z, w] }, ... },
//   morph: { "skin_R": [corrective shape weights] },
//   hands: { "right": { joints, morph }, "ana.left": { joints, morph }, ... } }.
// `joints` and `morph` hold the default person's right and left hands (one mesh each); with
// more people, load one mesh per hand and pose each from `hands[name]` (`handBones`).

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
      // The root joint of a smaller or larger person's hand scales the skeleton.
      const s = b.s ?? 1;
      node.scale.set(s, s, s);
    }
    root.updateMatrixWorld(true);
  };
}

/** One hand's bones ("right", "ana.left") out of a pose's, for `handRig(oneMesh)(...)`. */
export function handBones(bones, hand) {
  return bones.hands?.[hand] ?? (hand === 'right' || hand === 'left' ? bones : undefined);
}

/** The hands a pose's bones name, e.g. ["right", "left", "ana.right"]. */
export function handNames(bones) {
  return Object.keys(bones.hands ?? {});
}

/** The side of a hand name ("ana.left" -> "left"): which mesh (hand_R / hand_L) shows it. */
export function handSide(hand) {
  return hand.endsWith('left') ? 'left' : 'right';
}

/** The default skin look: vertex colors (skin tone, lighter nails), soft roughness. */
export function skinMaterial(THREE, opts = {}) {
  return new THREE.MeshStandardMaterial({ vertexColors: true, roughness: opts.roughness ?? 0.55, metalness: 0 });
}
