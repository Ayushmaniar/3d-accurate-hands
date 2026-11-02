#!/bin/sh
# Rebuilds wasm/ from crates/handkit-wasm. Needs the wasm32-unknown-unknown target and
# wasm-bindgen-cli at the version pinned in crates/handkit-wasm/Cargo.toml.
set -e
here=$(cd "$(dirname "$0")" && pwd)
root="$here/../.."
cargo build --release --target wasm32-unknown-unknown -p handkit-wasm --manifest-path "$root/Cargo.toml"
wasm-bindgen --target web --no-typescript --out-dir "$here/wasm" "$root/target/wasm32-unknown-unknown/release/handkit_wasm.wasm"
