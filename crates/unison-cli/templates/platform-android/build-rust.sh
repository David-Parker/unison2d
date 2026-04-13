#!/bin/bash
#
# Build the Rust game library for Android targets.
#
# Prerequisites:
#   - Android NDK installed (via Android Studio SDK Manager)
#   - cargo-ndk: cargo install cargo-ndk
#   - Rust targets: rustup target add aarch64-linux-android x86_64-linux-android
#
# Usage:
#   ./build-rust.sh           # Release build (default)
#   ./build-rust.sh debug     # Debug build

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Auto-detect ANDROID_NDK_HOME if not set
if [ -z "${ANDROID_NDK_HOME:-}" ]; then
    NDK_DIR="$HOME/Library/Android/sdk/ndk"
    if [ -d "$NDK_DIR" ]; then
        ANDROID_NDK_HOME="$(ls -d "$NDK_DIR"/*/ 2>/dev/null | sort -V | tail -1)"
        ANDROID_NDK_HOME="${ANDROID_NDK_HOME%/}"
        export ANDROID_NDK_HOME
        echo "Auto-detected NDK: $ANDROID_NDK_HOME"
    else
        echo "Error: ANDROID_NDK_HOME not set and no NDK found in $NDK_DIR"
        exit 1
    fi
fi

PROFILE="${1:-release}"
CARGO_FLAGS=""
if [ "$PROFILE" = "release" ]; then
    CARGO_FLAGS="--release"
    RUST_PROFILE="release"
else
    RUST_PROFILE="debug"
fi

TARGETS="aarch64-linux-android x86_64-linux-android"
FEATURES="android"

# 16 KB page alignment required for Android 15+ devices
export RUSTFLAGS="-Clink-arg=-z -Clink-arg=max-page-size=16384"

echo "Building Rust library for Android ($PROFILE)..."

cd "$PROJECT_ROOT"

for target in $TARGETS; do
    echo "  Building for $target..."
    cargo ndk --target "$target" build $CARGO_FLAGS --features "$FEATURES" --no-default-features
done

# Map Rust target triples to Android ABI names
target_to_abi() {
    case "$1" in
        aarch64-linux-android) echo "arm64-v8a" ;;
        x86_64-linux-android)  echo "x86_64" ;;
        *) echo "Error: unknown target $1" >&2; exit 1 ;;
    esac
}

echo "Copying .so files to jniLibs..."
for target in $TARGETS; do
    abi="$(target_to_abi "$target")"
    src="$PROJECT_ROOT/target/$target/$RUST_PROFILE/lib{{CRATE_NAME}}.so"
    dst="$SCRIPT_DIR/app/src/main/jniLibs/$abi/lib{{CRATE_NAME}}.so"
    mkdir -p "$(dirname "$dst")"
    cp "$src" "$dst"
    echo "  $abi: $(du -h "$dst" | cut -f1)"
done

echo "Done."
