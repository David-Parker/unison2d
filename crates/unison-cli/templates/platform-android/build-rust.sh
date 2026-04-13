#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-release}"
ROOT_DIR="$(cd "$(dirname "$0")/../../" && pwd)"
LIB_NAME="lib{{CRATE_NAME}}.so"
OUT_DIR="$(cd "$(dirname "$0")/app/src/main/jniLibs" && pwd)"

cd "$ROOT_DIR"

if [ "$MODE" = "debug" ]; then
    CARGO_FLAGS=""
    BUILD_DIR="debug"
else
    CARGO_FLAGS="--release"
    BUILD_DIR="release"
fi

for target in arm64-v8a armeabi-v7a x86_64; do
    cargo ndk --target "$target" build $CARGO_FLAGS --no-default-features --features android
    mkdir -p "$OUT_DIR/$target"
    cp "target/${target}/${BUILD_DIR}/${LIB_NAME}" "$OUT_DIR/$target/"
done
