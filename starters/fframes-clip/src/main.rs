use fframes::cli::clap; // the derive below expands to `clap::...`
use fframes::{EncoderOptions, RenderOptions, StaticMediaProvider, cli};
use fframes_clip::{FframesClipMedia, FframesClipVideo, HEIGHT, LookArgs, WIDTH};
use fframes_skia_renderer::{SkiaFFramesRenderer, SkiaPipelineConcurrencyPolicy, SkiaPipelineConfig, vulkan::SkiaVulkanCtx};
use std::process::ExitCode;

/// Flags of this video next to the standard ones of `fframes::cli` (render, frame, strip,
/// inspect, ...). Run `cargo run --release -- --help`.
#[derive(Debug, clap::Args)]
struct VideoArgs {
    /// The Handkit clip to play.
    #[arg(long, default_value = "clip.json", global = true)]
    clip: std::path::PathBuf,
    /// A folder with hand_R.glb / hand_L.glb from `handkit mesh` (default: generate them).
    #[arg(long, global = true)]
    assets: Option<std::path::PathBuf>,
    /// The backdrop color.
    #[arg(long, default_value = "#2b2e33", global = true)]
    background: String,
    /// A soft round light behind the hands: x,y,radius,color with x a fraction of the frame
    /// width (0 left, 1 right) and y and radius fractions of its height (0 top), e.g.
    /// 0.5,0.45,0.28,#e8b4b0 centred.
    #[arg(long, global = true)]
    spotlight: Option<String>,
    /// How the hands are drawn, over the clip's own "look": a style (realistic, toon,
    /// spiderverse, arcane, clay), then key=value pairs, e.g. toon,skin=tan,outline=4
    /// (keys: skin, nail, outline, motion_blur, shutter, halftone, misregister, texture,
    /// subsurface, arm; skin is light, medium, tan, brown, dark or #rrggbb; arm is bare or
    /// fade, a sleeve through --look-json).
    #[arg(long, global = true)]
    look: Option<String>,
    /// The look as JSON, e.g. '{"style": "clay", "skin": "#c68e6e"}'.
    #[arg(long, global = true)]
    look_json: Option<String>,
    /// Motion blur: average N sub-frame renders across the shutter (180 degrees unless the look
    /// sets "shutter").
    #[arg(long, global = true)]
    motion_blur: Option<u32>,
    /// A looks demo: cycle through these looks (separated by ';', each like --look), each
    /// shown on its own backdrop with its name, e.g. "realistic;toon;spiderverse;arcane;clay".
    #[arg(long, global = true)]
    looks: Option<String>,
    /// Seconds per look of --looks.
    #[arg(long, default_value_t = 2.0, global = true)]
    look_every: f64,
}

fn main() -> ExitCode {
    let args = cli::parse::<VideoArgs>();
    let media = FframesClipMedia::prepare().expect("media");
    let text = match std::fs::read_to_string(&args.app.clip) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: reading {}: {e}", args.app.clip.display());
            return ExitCode::FAILURE;
        }
    };
    let look = LookArgs {
        look: args.app.look.clone(),
        look_json: args.app.look_json.clone(),
        motion_blur: args.app.motion_blur,
        looks: args.app.looks.clone(),
        look_every: args.app.look_every,
    };
    let video = match FframesClipVideo::new(&media, &text, args.app.assets.as_deref(), &args.app.background, args.app.spotlight.as_deref(), &look) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {}: {e}", args.app.clip.display());
            return ExitCode::FAILURE;
        }
    };
    let gpu = SkiaVulkanCtx::new(WIDTH, HEIGHT).expect("GPU context");

    cli::new(
        &video,
        RenderOptions {
            media: Some(&media),
            video_encoder_options: EncoderOptions {
                preferred_encoder: Some("libx264"),
                codec_params: Some(&[("crf", "18"), ("preset", "medium")]),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .args(args)
    .backend(
        SkiaFFramesRenderer::new_vulkan(&gpu, SkiaPipelineConfig { concurrency_policy: SkiaPipelineConcurrencyPolicy::MaxPerformance, ..Default::default() })
            .expect("skia renderer"),
    )
    .preview(fframes_native_player::cli_preview)
    .run()
}
