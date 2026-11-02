// The Handkit core in the browser (WebAssembly): the same pose files as the CLI.
//   import { init, bones } from '@handkit/three/core.js';
//   await init();
//   handRig(gltf.scene)(JSON.parse(bones(poseText)));
import initWasm, * as wasm from './wasm/handkit_wasm.js';

let ready;
/** Loads the WebAssembly module once; pass a URL or bytes to load it from somewhere else. */
export function init(source) {
  ready ??= initWasm(source ? { module_or_path: source } : undefined);
  return ready;
}

export const { bones, query, solve, gesture, look, Clip } = wasm;
