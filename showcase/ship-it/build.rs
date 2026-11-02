//! On Windows, copies FFmpeg's DLLs (from `FFMPEG_DIR`) next to the binary, so it runs without
//! FFmpeg's `bin` on PATH. Without them Windows stops the program before it prints anything,
//! even `--help`.
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=FFMPEG_DIR");
    if std::env::var_os("CARGO_CFG_WINDOWS").is_none() {
        return;
    }
    let Some(ffmpeg) = std::env::var_os("FFMPEG_DIR") else { return };
    // OUT_DIR is <target>/<profile>/build/<crate>-<hash>/out.
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let Some(bin_dir) = out.ancestors().nth(3) else { return };
    let Ok(entries) = std::fs::read_dir(PathBuf::from(ffmpeg).join("bin")) else { return };
    for e in entries.flatten() {
        let from = e.path();
        if from.extension().is_none_or(|x| !x.eq_ignore_ascii_case("dll")) {
            continue;
        }
        let to = bin_dir.join(e.file_name());
        let same = std::fs::metadata(&to).ok().zip(e.metadata().ok()).is_some_and(|(a, b)| a.len() == b.len());
        if !same {
            if let Err(err) = std::fs::copy(&from, &to) {
                println!("cargo:warning=could not copy {}: {err}", from.display());
            }
        }
    }
}
