# M5 plan: a realistic hand, and looks

The user's priorities for M5: a hand that reads as real in close-up video, then swappable looks
(skin tones, toon / Spider-Verse, Arcane painterly, clay). No muscle layer.

## 1. Shape (handkit-mesh/src/sdf.rs)

The current hand reads as a soft rubber glove: the palm is a thick box, the fingers are round
tubes of almost even thickness, there are no knuckle bones, pads, nails or tendons. Real hands:

- Fingers are wider than they are deep, flatter on the back, with a fleshy pad on the palm side
  of each segment, narrower at the joints (the pip joint shows a bony bump on the back), and
  taper to the tip; the nail sits on a flattened back of the last segment.
- The palm is thin at the centre and thick at the heel; the back of the hand shows the
  metacarpal knuckle heads as bumps when the fingers bend, and faint extensor tendons fanning
  from the wrist to the knuckles. The web between the fingers sits well above the palm knuckles.
- The thumb's base is a big fleshy pad (thenar) and the web between thumb and index is thin.
- The wrist is flatter than the forearm, with the ulnar bone's bump on the pinky side; the
  forearm is round near the elbow (muscle) and flat near the wrist.
- Creases: palm lines (heart, head, life) and the flexion creases under each finger joint are
  shallow grooves; back-of-knuckle wrinkles are detail for the normal map, not geometry.

Nails get their own region (`nail` already exists per vertex) so looks shade them apart.

## 2. Surface (UVs and maps)

- UVs: charts per bone, packed in one atlas, with every texel's colour computed from its
  bind-pose 3D position (solid texturing). Both sides of a seam sample the same 3D function,
  so seams don't show. Dilate charts by a few texels.
- Maps (2048): normal (pores from the generated tiles via triplanar sampling, knuckle wrinkles,
  palm ridges), roughness, and a mask map (R redness: knuckles, fingertips, palm; G nail; B cavity
  / creases; A fine albedo variation). The albedo for a tone is computed from the masks at load,
  so any skin tone is a parameter, not a texture.

## 3. Lighting and render

- fframes: Material::standard with the maps, Environment::studio plus a warm key and a cool rim.
  A screen-space subsurface pass on the CPU image (blur the lit hand with a red-weighted kernel
  inside the hand's alpha, mix ~30 %) gives soft, fleshy skin.
- Motion blur (both gate agents missed it): average N sub-frame renders.

## 4. Looks

A pose or clip may carry `"look": {"style": ..., "skin": "#c68e6e" | "light" | ..., ...}`.
Styles: realistic, toon, spiderverse (cel + ink outline + halftone + misregistration), arcane
(painted albedo + soft cel + line + canvas), clay. Stylized looks are a post-process over two
renders per frame: colour and normals (Material::normals): cel bands from N·L, outlines from
normal and alpha edges, halftone from screen-space dots.

three.js gets the same looks (MeshPhysicalMaterial for realistic; a post pass for stylized).

## Images

`gen/` holds references and textures made with image generation (codex): ref_back/palm/side
(proportions), tile_* (detail), nail, style_* (targets for the stylized looks).
