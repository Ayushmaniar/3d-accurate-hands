use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};
use clap::{Parser, Subcommand};

/// Pose, check and preview the Handkit hand rig.
#[derive(Parser)]
#[command(name = "handkit", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Limits, contacts, penetrations, site gaps and aim for a pose, as text (or --json).
    Query {
        pose: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Every bone's joint position and axes, and every site.
    Fk { pose: PathBuf },
    /// The conventions and every parameter, or one parameter (e.g. thumb.cmc_rot).
    Explain { key: Option<String> },
    /// Capsules, nails and sites in world space as JSON, for renderers.
    Geometry { pose: PathBuf },
    /// A multi-view PNG preview (needs node; uses tools/preview).
    Render {
        pose: PathBuf,
        /// Output PNG (default: next to the pose file).
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// Comma-separated: front, side, top, back, three_quarter, hero (the pose's camera).
        #[arg(long, default_value = "front,side,top,hero")]
        views: String,
        /// Pixel size of each view.
        #[arg(long, default_value_t = 384)]
        size: u32,
    },
}

fn load(path: &Path) -> Result<handkit_core::Solved> {
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    handkit_core::solve(&text).map_err(|e| anyhow!(e))
}

fn preview_script() -> PathBuf {
    std::env::var_os("HANDKIT_PREVIEW")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/preview/render.mjs"))
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Query { pose, json } => {
            let report = load(&pose)?.query().map_err(|e| anyhow!(e))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report.json())?);
            } else {
                print!("{}", report.text());
            }
        }
        Cmd::Fk { pose } => print!("{}", load(&pose)?.fk_text()),
        Cmd::Explain { key } => match key {
            Some(k) => println!("{}", handkit_core::explain::one(&k).map_err(|e| anyhow!(e))?),
            None => println!("{}", handkit_core::explain::all()),
        },
        Cmd::Geometry { pose } => println!("{}", serde_json::to_string(&load(&pose)?.geometry())?),
        Cmd::Render { pose, out, views, size } => {
            let solved = load(&pose)?;
            let out = out.unwrap_or_else(|| pose.with_extension("png"));
            let geom = std::env::temp_dir().join(format!("handkit-geom-{}.json", std::process::id()));
            std::fs::write(&geom, serde_json::to_string(&solved.geometry())?)?;
            let script = preview_script();
            let status = Command::new("node")
                .arg(&script)
                .arg(&geom)
                .arg(&out)
                .arg(&views)
                .arg(size.to_string())
                .status()
                .with_context(|| format!("running node {}", script.display()))?;
            let _ = std::fs::remove_file(&geom);
            if !status.success() {
                bail!("preview renderer failed");
            }
            let report = solved.query().map_err(|e| anyhow!(e))?;
            println!(
                "wrote {} ({} limit violations, {} penetrations, {} contacts; run `handkit query` for details)",
                out.display(),
                report.violations.len(),
                report.penetrations.len(),
                report.contacts.len()
            );
        }
    }
    Ok(())
}
