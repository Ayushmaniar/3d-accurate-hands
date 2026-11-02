# remotion-clip

Plays a Handkit clip in Remotion: `<HandkitClip clip={...} look={...} />` (`src/HandkitClip.jsx`)
draws the hands in a `@remotion/three` canvas. Each frame samples the clip (the WebAssembly core) at
`useCurrentFrame() / fps`, poses the glb hands and renders them in the look with
`@handkit/three/look.js`, so frames render in any order and on any worker. The demo composition
(`src/index.jsx`) plays `clips/demo_basics.json` on a plain backdrop.

```bash
npm install
npm run assets            # copies ../../assets/hand_R.glb and hand_L.glb into public/
npm run studio            # preview
npx remotion render src/index.jsx HandkitDemo out/demo.mp4 --gl=angle
npx remotion render src/index.jsx HandkitDemo out/spider.mp4 --props='{"look":"spiderverse"}'
```

- `clip`: a clip object or its JSON text; `look`: a look (`{ "style": "toon", "skin": "tan" }` or a
  style name, `handkit explain looks`), default the clip's own; `assets`: the glb files under
  `public/` by side. The canvas is transparent: put the backdrop behind it.
- Every hand of the clip is drawn (`right`, `left`, `ana.right`, ...: one glb each, posed from
  `handBones(bones, name)`), each person in their own skin and nails from the clip's `"people"`.
- The look needs WebGL2: `--gl=angle` uses the GPU (set in `remotion.config.js`); `swiftshader`
  works without one, slower. `--browser-executable` can point at a local Chrome Headless Shell.
- A 1920x1080 frame of the demo renders in about 0.1 s here (156 frames in 24 s with bundling).
- Dependencies are pinned to exact versions. `@handkit/three` is linked from `packages/`
  (`remotion.config.js` keeps webpack from following the link, so it finds this `three`).
