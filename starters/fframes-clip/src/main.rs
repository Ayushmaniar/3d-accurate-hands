use fframes::cli::clap; // the derive below expands to `clap::...`
use fframes::{EncoderOptions, RenderOptions, StaticMediaProvider, cli};
use fframes_clip::{FframesClipMedia, FframesClipVideo, HEIGHT, WIDTH};
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
    #[arg(long, default_value = "#2b2e33", global = true)]
    background: String,
    /// A soft round light behind the hands: x,y,radius,color with x, y and radius as
    /// fractions of the frame height (e.g. 0.3,0.45,0.28,#e8b4b0).
    #[arg(long, global = true)]
    spotlight: Option<String>,
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
    let video = match FframesClipVideo::new(&media, &text, args.app.assets.as_deref(), &args.app.background, args.app.spotlight.as_deref()) {
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
