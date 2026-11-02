# 3D Accurate Hands (working name: Handkit)

An anatomically constrained, agent-drivable 3D hand and forearm rig for code-made video. One Rust core runs natively in fframes and as WebAssembly in HyperFrames and Remotion. AI agents pose it in joint angles, check it with numbers and small renders, and animate it with natural, deterministic motion.

Design doc: https://claude.ai/artifact/MxBq9Ef5StLhakD7BpCRBa

## Layout

- `experiments/joint-angles/`: the first experiment. A capsule hand rig (`hand.js`), a headless multi-view renderer (`node render.mjs pose.json`), the agent-facing spec and tasks, and three agents' attempts at 7 gestures under `runs/`.
- `Reference videos to Animate/`: hand-choreography videos for the final test (an agent recreates them with the library). Video files stay local; each `clean/<slug>/NOTES.md` describes its segments.

## Running the experiment renderer

```bash
cd experiments/joint-angles
npm install
node render.mjs runs/loop-opus/T6.json
```

It uses a Playwright Chromium found under `%LOCALAPPDATA%/ms-playwright`.
