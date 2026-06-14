#!/usr/bin/env bash
set -euo pipefail

# Build Rust core for macOS, generate Swift bindings, copy to project.
echo "==> Building Rust core"
cargo build --release
ls -lh target/release/libmaccy_core.a target/release/libmaccy_sync.a

echo "==> Generating UniFFI Swift bindings"
cargo run --release --bin uniffi-bindgen --package maccy-core generate \
  --library target/release/libmaccy_core.dylib \
  --language swift \
  --out-dir Maccy/Generated

cp Maccy/Generated/MaccyCore.swift Maccy/MaccyCore.swift
echo "==> Swift bindings generated"
