# hyperframes-clip

Plays a Handkit clip in a HyperFrames composition (`index.html`): a three.js canvas over a
backdrop, the hands posed from `clip.bones(t)` for the time HyperFrames seeks to (`hf-seek`), drawn
in a look with `@handkit/three/look.js`. Nothing carries over from the frame before, so parallel
workers and scrubbing in Studio give the same frames. The core, the hands and the look's textures
load behind `window.__hf.buildReady` before the first frame.

```bash
npm install
npm run assets            # copies ../../assets/hand_R.glb and hand_L.glb into assets/
npm run dev               # Studio preview
npx hyperframes@0.8.179 render --output out/demo.mp4
npx hyperframes@0.8.179 render --variables '{"look":"spiderverse"}' --output out/spider.mp4
```

- `clip.json` is the clip (a copy of `clips/demo_basics.json`); set the root's `data-duration` to its
  duration. Variables: `look` (a style name or look JSON; empty takes the clip's own) and
  `background`.
- `three` and `@handkit/three` come from `node_modules` through the import map (no network at
  render time).
- The 156-frame demo renders in about 17 s (hardware GPU, 4 workers).
