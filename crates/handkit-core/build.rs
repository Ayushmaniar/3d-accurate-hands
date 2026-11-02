// Embeds every gestures/*.json file so the library ships inside the crate.
use std::{env, fs, path::Path};

fn main() {
    let dir = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("gestures");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut entries: Vec<(String, String)> = fs::read_dir(&dir)
        .map(|rd| {
            rd.filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "json"))
                .map(|p| (p.file_stem().unwrap().to_string_lossy().into_owned(), p.display().to_string()))
                .collect()
        })
        .unwrap_or_default();
    entries.sort();
    let body: String = entries
        .iter()
        .map(|(name, path)| {
            println!("cargo:rerun-if-changed={path}");
            format!("    ({name:?}, include_str!({path:?})),\n")
        })
        .collect();
    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("gestures.rs");
    fs::write(out, format!("pub static GESTURES: &[(&str, &str)] = &[\n{body}];\n")).unwrap();
}
