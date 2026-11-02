// <HandkitClip clip={...} look={...} />: a Handkit clip played in Remotion through @remotion/three.
// Each frame samples the clip (WebAssembly core) at useCurrentFrame() / fps, poses the glb hands
// and draws them in the look (@handkit/three/look.js), so any frame renders on its own. Every
// hand of the clip is drawn ("right", "left", "ana.right", ...: one mesh each), each person in
// their own skin and nails from the clip's "people".
import { useEffect, useRef, useState } from 'react';
import { useFrame, useThree } from '@react-three/fiber';
import { ThreeCanvas } from '@remotion/three';
import { staticFile, useCurrentFrame, useDelayRender, useVideoConfig } from 'remotion';
import * as THREE from 'three';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { handBones, handRig, handSide } from '@handkit/three';
import { init, Clip } from '@handkit/three/core.js';
import { HandkitLook } from '@handkit/three/look.js';

const DEFAULT_ASSETS = { right: 'hand_R.glb', left: 'hand_L.glb' };
const DEFAULT_CAMERA = { position: [0, 0.1, 0.6], target: [0, 0.05, 0], fov: 35 };

/**
 * @param clip   a clip (object or JSON text, see docs/POSE_SPEC.md, Clips)
 * @param look   a look ({ style, skin, ... } or a style name); default: the clip's own
 * @param assets the hand .glb files under public/ ({ right, left }: by side, for every person)
 * @param style  CSS for the canvas (it is transparent: put a backdrop behind it)
 */
export const HandkitClip = ({ clip, look = null, assets = DEFAULT_ASSETS, style }) => {
  const { width, height } = useVideoConfig();
  return (
    <ThreeCanvas width={width} height={height} style={style} dpr={1} gl={{ antialias: false, alpha: true, preserveDrawingBuffer: true }}>
      <Hands clip={clip} look={look} assets={assets} />
    </ThreeCanvas>
  );
};

/** The colours a hand wears when its person has their own look (else the look's: null). */
export function personColors(clipObject, looks, hand) {
  const person = hand.includes('.') ? hand.slice(0, hand.lastIndexOf('.')) : '';
  return clipObject.people?.[person]?.look ? looks[hand] : null;
}

const Hands = ({ clip, look, assets }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const { gl, scene, camera, advance } = useThree();
  const { delayRender, continueRender, cancelRender } = useDelayRender();
  const [ready, setReady] = useState(null);
  const hold = useRef(null);
  const text = typeof clip === 'string' ? clip : JSON.stringify(clip);
  const lookKey = JSON.stringify(look);

  useEffect(() => {
    hold.current = delayRender('Loading the Handkit core, hands and look');
    let alive = true;
    const roots = [];
    let styled = null;
    (async () => {
      await init();
      const c = new Clip(text);
      const loader = new GLTFLoader();
      const parsed = JSON.parse(text);
      // One mesh per hand of the clip, by its side; each posed from its own bones.
      const names = JSON.parse(c.hands);
      const looks = JSON.parse(c.looks());
      const gltfs = await Promise.all(names.map((h) => { const s = handSide(h); return loader.loadAsync(staticFile(assets[s] ?? DEFAULT_ASSETS[s])); }));
      if (!alive) return;
      gltfs.forEach((g) => { scene.add(g.scene); roots.push(g.scene); });
      const rigs = gltfs.map((g, i) => { const apply = handRig(g.scene); return (bones) => apply(handBones(bones, names[i])); });
      styled = new HandkitLook(gl, look ?? JSON.parse(c.look));
      for (const [i, g] of gltfs.entries()) await styled.dress(g, personColors(parsed, looks, names[i]));
      const cam = JSON.parse(c.camera) ?? DEFAULT_CAMERA;
      camera.fov = cam.fov ?? 35;
      camera.near = 0.01;
      camera.far = 50;
      camera.position.set(...cam.position);
      camera.lookAt(new THREE.Vector3(...cam.target));
      camera.updateProjectionMatrix();
      if (alive) setReady({ clip: c, rigs, look: styled });
    })().catch((e) => cancelRender(e));
    return () => {
      alive = false;
      roots.forEach((r) => scene.remove(r));
      styled?.dispose();
      if (hold.current !== null) { continueRender(hold.current); hold.current = null; }
    };
  }, [text, lookKey]); // eslint-disable-line react-hooks/exhaustive-deps

  // Once loaded, draw the current frame before letting Remotion take it.
  useEffect(() => {
    if (!ready || hold.current === null) return;
    advance(performance.now());
    continueRender(hold.current);
    hold.current = null;
  }, [ready]); // eslint-disable-line react-hooks/exhaustive-deps

  const t = frame / fps;
  useFrame(() => {
    if (!ready) return;
    const { clip: c, rigs, look: styled } = ready;
    // Seek-safe: the pose comes from the clip at this frame's time (plus a sub-frame offset
    // for motion blur), never from the previous frame.
    const pose = (dt) => {
      const at = Math.min(Math.max(t + dt / fps, 0), c.duration);
      const bones = JSON.parse(c.bones(at));
      rigs.forEach((apply) => apply(bones));
    };
    pose(0);
    styled.render(scene, camera, styled.look.motion_blur > 1 ? pose : null);
  }, 1);
  return null;
};

/** The clip's length in frames at `fps` (its "duration", else the last key + 0.5 s). */
export function clipFrames(clipObject, fps) {
  const keys = clipObject.keys ?? [];
  const duration = clipObject.duration ?? (keys.length ? Math.max(...keys.map((k) => k.t)) + 0.5 : 1);
  return Math.max(1, Math.round(duration * fps));
}
