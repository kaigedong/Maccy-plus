#!/usr/bin/env bash
set -euo pipefail

# Build Rust core for Android (aarch64), generate Kotlin bindings, fix generated code.
ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/27.0.12077973}"
export ANDROID_NDK_HOME

echo "ANDROID_HOME=$ANDROID_HOME"
echo "ANDROID_NDK_HOME=$ANDROID_NDK_HOME"
ls "$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/" | head -5

echo "==> Building Rust for arm64-v8a"
cargo ndk \
  -t arm64-v8a \
  -o android/app/src/main/jniLibs \
  build --release \
  --package maccy-core

echo "==> Generating UniFFI Kotlin bindings"
LIB="android/app/src/main/jniLibs/arm64-v8a/libmaccy_core.so"
ls -lh "$LIB"

cargo run --release \
  --bin uniffi-bindgen \
  --package maccy-core \
  generate \
  --library "$LIB" \
  --language kotlin \
  --out-dir android/app/src/main/java

echo "Generated Kotlin files:"
find android/app/src/main/java -name "*.kt" -o -name "*.java" | sort

echo "==> Fixing UniFFI generated code"
# CoreError subclasses have 'val `message`' that shadows Throwable.message.
# Remove 'val' from constructor param so 'override val message' getter works alone.
sed -i 's/val `message`: kotlin\.String/`message`: kotlin.String/g' \
  android/app/src/main/java/com/kaigedong/maccy/maccy_core.kt
echo "✅ Android Rust build complete"
