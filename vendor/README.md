# Vendored crates

## fframes_skia_renderer 1.3.0 (MIT, https://github.com/dmtrKovalenko/fframes)

An unmodified copy of the crates.io release except for one fix in
`src/backends/vulkan_readback.rs`: the readback's copy command buffer starts with a pipeline
barrier that waits for every earlier write to the frame image. Vulkan lets separate submits on
one queue overlap, so without it the copy could start while Skia was still drawing and read
some rows of the previous frame: about one frame in a few hundred came out torn into
horizontal stripes. Players and showcases patch it in with `[patch.crates-io]`. Drop this copy
once fframes ships the fix.
