#!/bin/bash
# Build script for Rust core library for Android
# Compiles libmepassa_core for Android devices and emulators

set -e

echo "🔨 Building Rust core library for Android..."
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Project root
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CORE_DIR="$PROJECT_ROOT/core"
ANDROID_DIR="$PROJECT_ROOT/android"
JNILIBS_DIR="$ANDROID_DIR/app/src/main/jniLibs"

echo -e "${BLUE}Project root:${NC} $PROJECT_ROOT"
echo -e "${BLUE}Core directory:${NC} $CORE_DIR"
echo -e "${BLUE}Android directory:${NC} $ANDROID_DIR"
echo ""

# Create jniLibs directories
mkdir -p "$JNILIBS_DIR/armeabi-v7a"
mkdir -p "$JNILIBS_DIR/arm64-v8a"
mkdir -p "$JNILIBS_DIR/x86_64"

# Check if Android targets are installed
echo -e "${YELLOW}Checking Rust Android targets...${NC}"
TARGETS_NEEDED=(
    "armv7-linux-androideabi"
    "aarch64-linux-android"
    "x86_64-linux-android"
)

for target in "${TARGETS_NEEDED[@]}"; do
    if ! rustup target list --installed | grep -q "$target"; then
        echo -e "${YELLOW}Installing $target...${NC}"
        rustup target add "$target"
    else
        echo -e "  ✅ $target already installed"
    fi
done
echo ""

# Set Android NDK environment variables for C compiler (required by ring and other crates with C dependencies)
NDK_PATH="/Users/edsonmartins/Library/Android/sdk/ndk/26.3.11579264"
TOOLCHAIN_PATH="$NDK_PATH/toolchains/llvm/prebuilt/darwin-x86_64/bin"
export AR="$TOOLCHAIN_PATH/llvm-ar"

# Build for Android ARM64 (64-bit ARM - most modern devices)
echo -e "${GREEN}Building for Android ARM64 (aarch64-linux-android)...${NC}"
cd "$CORE_DIR"
export CC_aarch64_linux_android="$TOOLCHAIN_PATH/aarch64-linux-android24-clang"
export CXX_aarch64_linux_android="$TOOLCHAIN_PATH/aarch64-linux-android24-clang++"
cargo build --release --target aarch64-linux-android --features voip -p mepassa-core
echo ""

# Build for Android ARMv7 (32-bit ARM - older devices)
echo -e "${GREEN}Building for Android ARMv7 (armv7-linux-androideabi)...${NC}"
export CC_armv7_linux_androideabi="$TOOLCHAIN_PATH/armv7a-linux-androideabi24-clang"
export CXX_armv7_linux_androideabi="$TOOLCHAIN_PATH/armv7a-linux-androideabi24-clang++"
cargo build --release --target armv7-linux-androideabi --features voip -p mepassa-core
echo ""

# Build for Android x86_64 (emulators)
echo -e "${GREEN}Building for Android x86_64 (x86_64-linux-android)...${NC}"
export CC_x86_64_linux_android="$TOOLCHAIN_PATH/x86_64-linux-android24-clang"
export CXX_x86_64_linux_android="$TOOLCHAIN_PATH/x86_64-linux-android24-clang++"
cargo build --release --target x86_64-linux-android --features voip -p mepassa-core
echo ""

# Copy libraries to jniLibs
echo -e "${GREEN}Copying libraries to jniLibs...${NC}"

cp "$PROJECT_ROOT/target/aarch64-linux-android/release/libmepassa_core.so" \
   "$JNILIBS_DIR/arm64-v8a/libmepassa_core.so"
echo -e "  ✅ arm64-v8a/libmepassa_core.so"

cp "$PROJECT_ROOT/target/armv7-linux-androideabi/release/libmepassa_core.so" \
   "$JNILIBS_DIR/armeabi-v7a/libmepassa_core.so"
echo -e "  ✅ armeabi-v7a/libmepassa_core.so"

cp "$PROJECT_ROOT/target/x86_64-linux-android/release/libmepassa_core.so" \
   "$JNILIBS_DIR/x86_64/libmepassa_core.so"
echo -e "  ✅ x86_64/libmepassa_core.so"
echo ""

# Show library sizes
echo -e "${GREEN}Library sizes:${NC}"
du -h "$JNILIBS_DIR"/*/*.so
echo ""

echo -e "${GREEN}✅ Build complete!${NC}"
echo ""
echo -e "${BLUE}Native libraries are ready in:${NC}"
echo "  $JNILIBS_DIR"
echo ""
echo -e "${BLUE}Next steps:${NC}"
echo "  1. Build Android app: cd android && ./gradlew assembleDebug"
echo "  2. Install on device/emulator: adb install app/build/outputs/apk/debug/app-debug.apk"
