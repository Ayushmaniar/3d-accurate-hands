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
    /// Change joint values until the file's goals hold, and write them back. Exits with code 2
    /// (after writing) when a goal is still missed.
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
    /// Each skeleton joint's local rotation and translation for a pose, as JSON for mesh adapters.
    Bones { pose: PathBuf },
    /// Generate the procedural hand as a skinned binary glTF (.glb).
    Mesh {
        /// right or left.
        #[arg(long, default_value = "right")]
        side: String,
        #[arg(short, long)]
        out: PathBuf,
        /// Grid spacing in millimeters (smaller = finer, slower).
        #[arg(long, default_value_t = 1.2)]
        voxel_mm: f64,
    },
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
        /// Draw the skinned hand mesh instead of the capsules (generates assets/hand_*.glb once).
        #[arg(long)]
        mesh: bool,
    },
    /// A clip's pose at one time, as a pose file (or --bones for mesh adapters).
    Sample {
        clip: PathBuf,
        /// Seconds.
        t: f64,
        #[arg(long)]
        bones: bool,
        /// Write each hand's placement as wrist, palm_facing and fingers_toward (the clip
        /// works in elbow + rotation).
        #[arg(long)]
        wrist: bool,
    },
    /// Motion problems over a whole clip: held goals coming apart, penetrations mid-move, joints
    /// moving too fast, pops, fingertips leaving the camera's view.
    Check { clip: PathBuf },
    /// A contact sheet of a clip: frames at evenly spaced times in one labelled PNG.
    Sheet {
        clip: PathBuf,
        /// Output PNG (default: next to the clip, .sheet.png).
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// How many frames.
        #[arg(short, default_value_t = 12)]
        n: usize,
        /// Comma-separated times in seconds instead of evenly spaced ones (e.g. 1,1.2,1.4).
        #[arg(long)]
        at: Option<String>,
        /// Width x height of each frame (default: 216 high, at the clip's aspect).
        #[arg(long)]
        size: Option<String>,
        #[arg(long)]
        mesh: bool,
        /// Background color as hex, e.g. "#2b2e33" (match the video player's).
        #[arg(long)]
        background: Option<String>,
    },
    /// A quick MP4 of a clip from the three.js preview (needs node and ffmpeg).
    Preview {
        clip: PathBuf,
        /// Output MP4 (default: next to the clip).
        #[arg(short, long)]
        out: Option<PathBuf>,
        #[arg(long, default_value = "960x540")]
        size: String,
        /// Draw the capsules instead of the skinned mesh.
        #[arg(long)]
        capsules: bool,
    },
}

fn load_clip(path: &Path) -> Result<handkit_core::clip::Clip> {
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    handkit_core::clip::Clip::parse(&text).map_err(|e| anyhow!(e))
}

type Frames = Vec<(f64, handkit_core::clip::Frame)>;

/// Samples a clip at many times on every core.
fn sample_frames(clip: &handkit_core::clip::Clip, times: &[f64]) -> Result<Frames> {
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(times.len().max(1));
    let chunk = times.len().div_ceil(threads).max(1);
    let parts: Vec<Result<Frames, String>> = std::thread::scope(|s| {
        let handles: Vec<_> = times
            .chunks(chunk)
            .map(|ts| s.spawn(move || ts.iter().map(|t| clip.sample(*t).map(|f| (*t, f)).map_err(|e| format!("t={t}: {e}"))).collect()))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let mut out = Vec::with_capacity(times.len());
    for p in parts {
        out.extend(p.map_err(|e| anyhow!(e))?);
    }
    Ok(out)
}

fn parse_size(s: &str) -> Result<(u32, u32)> {
    let (w, h) = s.split_once('x').context("size is WIDTHxHEIGHT, e.g. 960x540")?;
    Ok((w.trim().parse()?, h.trim().parse()?))
}

/// The renderer's input for clip frames: geometry (and bones, for meshes) per frame.
fn frames_spec(frames: &Frames, size: (u32, u32), mesh: bool) -> Result<serde_json::Value> {
    let mut out = Vec::new();
    let mut sides = Vec::new();
    for (t, f) in frames {
        let solved = handkit_core::solve_value(&f.pose).map_err(|e| anyhow!(e))?;
        let mut g = solved.geometry();
        g["label"] = serde_json::json!(format!("{t:.2} s"));
        if mesh {
            g["bones"] = solved.bones();
        }
        sides = solved.hands.iter().map(|h| h.side).collect();
        out.push(g);
    }
    let mut spec = serde_json::json!({ "frames": out, "size": [size.0, size.1] });
    if mesh {
        let mut files = serde_json::Map::new();
        for side in sides {
            files.insert(side.name().into(), serde_json::json!(mesh_asset(side)?));
        }
        spec["mesh"] = serde_json::json!({ "files": files });
    }
    Ok(spec)
}

/// Runs the preview renderer on a spec written to a temporary file.
fn run_renderer(spec: &serde_json::Value, out: &Path) -> Result<()> {
    let geom = std::env::temp_dir().join(format!("handkit-frames-{}.json", std::process::id()));
    std::fs::write(&geom, serde_json::to_string(spec)?)?;
    let script = preview_script();
    let status = Command::new("node").arg(&script).arg(&geom).arg(out).status().with_context(|| format!("running node {}", script.display()))?;
    let _ = std::fs::remove_file(&geom);
    if !status.success() {
        bail!("preview renderer failed");
    }
    Ok(())
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

/// The hand mesh for a side: `$HANDKIT_ASSETS/hand_R.glb` (default: the repo's assets/),
/// generated on first use.
fn mesh_asset(side: handkit_core::pose::Side) -> Result<PathBuf> {
    let dir = std::env::var_os("HANDKIT_ASSETS")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets"));
    let path = dir.join(format!("hand_{}.glb", if side == handkit_core::pose::Side::Left { "L" } else { "R" }));
    if !path.exists() {
        std::fs::create_dir_all(&dir)?;
        let mesh = handkit_mesh::generate(side, &handkit_mesh::Options::default());
        std::fs::write(&path, handkit_mesh::glb::to_glb(&mesh))?;
    }
    Ok(std::fs::canonicalize(path)?)
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
            let missed = report.goals.iter().filter(|g| !g.met).count();
            if missed > 0 {
                eprintln!("{missed} goal(s) missed; the pose is written as far as it got (exit code 2)");
                std::process::exit(2);
            }
        }
        Cmd::Sample { clip, t, bones, wrist } => {
            let c = load_clip(&clip)?;
            let mut f = c.sample(t).map_err(|e| anyhow!(e))?;
            if wrist {
                let solved = handkit_core::solve_value(&f.pose).map_err(|e| anyhow!(e))?;
                let r = |x: f64| (x * 1e4).round() / 1e4;
                for h in &solved.hands {
                    let Some(b) = h.bones.iter().find(|b| b.name == "hand") else { continue };
                    let (p, m) = (b.frame.translation, b.frame.matrix3);
                    f.pose["hands"][h.side.name()]["place"] = serde_json::json!({
                        "wrist": [r(p.x), r(p.y), r(p.z)],
                        "palm_facing": [r(m.z_axis.x), r(m.z_axis.y), r(m.z_axis.z)],
                        "fingers_toward": [r(m.y_axis.x), r(m.y_axis.y), r(m.y_axis.z)],
                    });
                }
            }
            if bones {
                let solved = handkit_core::solve_value(&f.pose).map_err(|e| anyhow!(e))?;
                emit(serde_json::to_string(&solved.bones())? + "\n");
            } else {
                let mut pose = f.pose;
                if !f.goals.is_empty() {
                    pose["goals"] = serde_json::json!(f.goals);
                }
                emit(handkit_core::pose::to_text(&pose));
            }
        }
        Cmd::Check { clip } => {
            let c = load_clip(&clip)?;
            // The frames a video of the clip renders.
            let times: Vec<f64> = (0..c.frames()).map(|i| i as f64 / c.fps).collect();
            let frames = sample_frames(&c, &times)?;
            let report = handkit_core::check::analyze(&c, &frames, c.aspect()).map_err(|e| anyhow!(e))?;
            emit(report.text());
        }
        Cmd::Sheet { clip, out, n, at, size, mesh, background } => {
            let c = load_clip(&clip)?;
            let times: Vec<f64> = match at {
                Some(list) => list.split(',').map(|s| s.trim().trim_end_matches('s').parse::<f64>()).collect::<Result<_, _>>().context("--at is a comma-separated list of seconds")?,
                None if n <= 1 => vec![0.0],
                None => (0..n).map(|i| c.duration * i as f64 / (n - 1) as f64).collect(),
            };
            let frames = sample_frames(&c, &times)?;
            let size = match size {
                Some(s) => parse_size(&s)?,
                None => (((216.0 * c.aspect() / 2.0).round() * 2.0) as u32, 216),
            };
            let mut spec = frames_spec(&frames, size, mesh)?;
            spec["cols"] = serde_json::json!(times.len().min(4));
            if let Some(bg) = background {
                let hex = bg.trim_start_matches('#');
                if hex.len() != 6 || u32::from_str_radix(hex, 16).is_err() {
                    bail!("--background is a hex color such as \"#2b2e33\"");
                }
                spec["background"] = serde_json::json!(format!("#{hex}"));
            }
            let out = out.unwrap_or_else(|| clip.with_extension("sheet.png"));
            run_renderer(&spec, &out)?;
            emit(format!("wrote {} ({} frames)\n", out.display(), times.len()));
        }
        Cmd::Preview { clip, out, size, capsules } => {
            let c = load_clip(&clip)?;
            let start = std::time::Instant::now();
            let times: Vec<f64> = (0..c.frames()).map(|i| i as f64 / c.fps).collect();
            let frames = sample_frames(&c, &times)?;
            let sampled = start.elapsed().as_secs_f64();
            let mut spec = frames_spec(&frames, parse_size(&size)?, !capsules)?;
            spec["video"] = serde_json::json!(true);
            let dir = std::env::temp_dir().join(format!("handkit-preview-{}", std::process::id()));
            run_renderer(&spec, &dir)?;
            let out = out.unwrap_or_else(|| clip.with_extension("mp4"));
            let ffmpeg = std::env::var_os("HANDKIT_FFMPEG").unwrap_or_else(|| "ffmpeg".into());
            let status = Command::new(ffmpeg)
                .args(["-y", "-loglevel", "error", "-framerate", &c.fps.to_string(), "-i"])
                .arg(dir.join("frame_%05d.png"))
                .args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "18"])
                .arg(&out)
                .status()
                .context("running ffmpeg (put it on PATH or set HANDKIT_FFMPEG)")?;
            let _ = std::fs::remove_dir_all(&dir);
            if !status.success() {
                bail!("ffmpeg failed");
            }
            emit(format!("wrote {} ({} frames; sampling {:.1} s, total {:.1} s)\n", out.display(), times.len(), sampled, start.elapsed().as_secs_f64()));
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
        Cmd::Bones { pose } => emit(serde_json::to_string(&load(&pose)?.bones())? + "
"),
        Cmd::Mesh { side, out, voxel_mm } => {
            let side = match side.as_str() {
                "right" => handkit_core::pose::Side::Right,
                "left" => handkit_core::pose::Side::Left,
                _ => bail!("--side is right or left"),
            };
            let start = std::time::Instant::now();
            let mesh = handkit_mesh::generate(side, &handkit_mesh::Options { voxel: voxel_mm / 1000.0 });
            let glb = handkit_mesh::glb::to_glb(&mesh);
            std::fs::write(&out, &glb)?;
            emit(format!(
                "wrote {} ({} vertices, {} triangles, {} joints, {:.0} KB, {:.1} s)
",
                out.display(),
                mesh.positions.len(),
                mesh.indices.len() / 3,
                mesh.skeleton.bones.len(),
                glb.len() as f64 / 1024.0,
                start.elapsed().as_secs_f64()
            ));
        }
        Cmd::Geometry { pose } => emit(serde_json::to_string(&load(&pose)?.geometry())? + "\n"),
        Cmd::Render { pose, out, views, size, mesh } => {
            let solved = load(&pose)?;
            let out = out.unwrap_or_else(|| pose.with_extension("png"));
            let geom = std::env::temp_dir().join(format!("handkit-geom-{}.json", std::process::id()));
            let mut g = solved.geometry();
            if mesh {
                let mut files = serde_json::Map::new();
                for h in &solved.hands {
                    files.insert(h.side.name().into(), serde_json::json!(mesh_asset(h.side)?));
                }
                g["mesh"] = serde_json::json!({ "bones": solved.bones(), "files": files });
            }
            // A clean transparent image for comparing renderers (tools/golden).
            if std::env::var_os("HANDKIT_GOLDEN").is_some() {
                g["golden"] = serde_json::json!(true);
            }
            // Studio light and a portrait frame with the wrist, for comparing with photos.
            if std::env::var_os("HANDKIT_STUDIO").is_some() {
                g["studio"] = serde_json::json!(true);
            }
            std::fs::write(&geom, serde_json::to_string(&g)?)?;
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
