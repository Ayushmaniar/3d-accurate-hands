# Vendored crates

## fframes_skia_renderer 1.3.0 (MIT, https://github.com/dmtrKovalenko/fframes)

A copy of the crates.io release with these fixes. Players and showcases patch it in with
`[patch.crates-io]`. Drop this copy once fframes ships them.

### The readback waits for the draws (`src/backends/vulkan_readback.rs`)

The readback's copy command buffer starts with a pipeline barrier that waits for every earlier
write to the frame image. Vulkan lets separate submits on one queue overlap, so without it the
copy could start while Skia was still drawing and read some rows of the previous frame.

### One GPU context per queue, on the discrete GPU (`src/backends/vulkan.rs`, `src/skia_pipeline.rs`)

Frames still came out corrupted, about one in 200: the hand layer of the frame rebuilt from
horizontal bands of other frames' hands, some grey (the luma of one frame with the chroma of
another). The cause was GPU contexts sharing a queue:

- The backend took the first device the Vulkan instance listed. On a laptop that is often the
  integrated GPU (here Intel UHD Graphics, with one graphics queue) rather than the discrete
  one (RTX 4070, 16 graphics queues).
- A render runs several Skia contexts at once (`MaxPerformance`: a third of the threads, 5 on
  this machine), spread over the device's queues. With one queue, all five shared it.
- A queue must only be used by one thread at a time. This crate locks the queue around its
  own submits, but Skia's work for one context could still interleave with another context's
  on the shared queue. Frames rendered through the GPU YUV conversion (`frame_export/planes.rs`,
  the default path) then read back planes mixed from different frames.

Measured on the clay look of the Pexels gestures clip (18 s, 546 frames per render): 3 of 4
renders had corrupted frames (9 frames in all) with five contexts on one queue, against none in
4 with a single context and none in 4 with the conversion done on the CPU.

The fix:

- `pick_physical_device` prefers a discrete GPU, then the device with the most graphics
  queues (a native driver over a layered one such as Microsoft's D3D12 translation).
  `FFRAMES_VULKAN_DEVICE` (part of a device name, e.g. `Intel`) picks one by hand.
- `SkiaBackend::gpu_queues` reports how many queues the contexts are spread over, and a
  render uses at most that many contexts, so no two of its contexts share a queue. On a GPU
  with one queue that is one context.

With it, every render came out clean: 8 of 8 of the clip's 12-16 s, 4 of 4 of the whole clip,
and 4 of 4 of the whole clip forced onto the Intel GPU (one context). On the RTX the 4 s range
renders in about 11 s instead of 18 s.
