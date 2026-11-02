use std::io::Write;
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
        /// List every site's position, not only the fingertips.
        #[arg(long)]
        sites: bool,
    },
    /// Change joint values until the file's goals hold, and write them back.
    Solve {
        pose: PathBuf,
        /// Output file (default: overwrite the pose file).
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// List the gesture library.
    Gestures,
    /// Print one library gesture as a pose file to start from.
    Gesture { name: String },
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

/// Prints to stdout; a reader that stops early (`| head`) is not an error.
fn emit(text: impl AsRef<str>) {
    let _ = std::io::stdout().lock().write_all(text.as_ref().as_bytes());
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Query { pose, json, sites } => {
            let report = load(&pose)?.query().map_err(|e| anyhow!(e))?;
            if json {
                emit(serde_json::to_string_pretty(&report.json(sites))? + "\n");
            } else {
                emit(report.text(sites));
            }
        }
        Cmd::Solve { pose, out } => {
            let text = std::fs::read_to_string(&pose).with_context(|| format!("reading {}", pose.display()))?;
            let raw: serde_json::Value = serde_json::from_str(&text).context("invalid JSON")?;
            let (solved, report) = handkit_core::solve::solve_goals(&raw).map_err(|e| anyhow!(e))?;
            let out = out.unwrap_or(pose);
            std::fs::write(&out, handkit_core::pose::to_text(&solved))?;
            emit(report.text() + &format!("wrote {}\n", out.display()));
        }
        Cmd::Gestures => {
            for (name, desc) in handkit_core::gestures::list() {
                emit(format!("{name}: {desc}\n"));
            }
        }
        Cmd::Gesture { name } => {
            let v = handkit_core::pose::gesture_value(&name).map_err(|e| anyhow!(e))?;
            emit(handkit_core::pose::to_text(&v));
        }
        Cmd::Fk { pose } => emit(load(&pose)?.fk_text()),
        Cmd::Explain { key } => match key {
            Some(k) => emit(handkit_core::explain::one(&k).map_err(|e| anyhow!(e))? + "\n"),
            None => emit(handkit_core::explain::all() + "\n"),
        },
        Cmd::Geometry { pose } => emit(serde_json::to_string(&load(&pose)?.geometry())? + "\n"),
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
