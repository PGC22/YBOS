#!/bin/bash
# build_android.sh — Cross-compile ybos-l0 for aarch64-linux-android
# WHY: Wrapper script to ensure NDK is present and build for Android target.

set -euo pipefail

echo "==> Starting ybos-l0 Cross-compile for Android (aarch64)"

# 1. Verify ANDROID_NDK_HOME
if [ -z "${ANDROID_NDK_HOME:-}" ]; then
    echo "ERROR: ANDROID_NDK_HOME is not set."
    echo "Please download the Android NDK and set the variable."
    echo "Example: export ANDROID_NDK_HOME=\$HOME/android-ndk-r26b"
    exit 1
fi

echo "==> Using NDK at: $ANDROID_NDK_HOME"

# 2. Identify host platform for toolchain path
OS_NAME=$(uname | tr '[:upper:]' '[:lower:]')
HOST_TAG="${OS_NAME}-x86_64"
TOOLCHAIN="$ANDROID_NDK_HOME/toolchain/bin" # Standard path in newer NDKs
# Fallback for some NDK structures
if [ ! -d "$TOOLCHAIN" ]; then
    TOOLCHAIN="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$HOST_TAG/bin"
fi

if [ ! -d "$TOOLCHAIN" ]; then
    echo "ERROR: Could not find toolchain at $TOOLCHAIN"
    exit 1
fi

# 3. Add TOOLCHAIN to PATH so cargo can find the linker defined in .cargo/config.toml
export PATH="$TOOLCHAIN:$PATH"

# 4. Add rust target
echo "==> Ensuring rust target aarch64-linux-android is installed..."
rustup target add aarch64-linux-android

# 5. Build
echo "==> Building ybos-l0 in release mode for aarch64-linux-android..."
cargo build --release --target aarch64-linux-android

# 6. Success
BINARY_PATH="target/aarch64-linux-android/release/ybos-l0"
if [ -f "$BINARY_PATH" ]; then
    echo "==> Build successful!"
    echo "==> Binary location: $BINARY_PATH"
    file "$BINARY_PATH"
else
    echo "ERROR: Build failed, binary not found."
    exit 1
fi

echo "Code implemented with help from AI Agents Claude, Codex, Jules."
