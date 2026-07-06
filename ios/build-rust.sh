#!/bin/bash
# Build script for Rust core library for iOS
# Compiles libzaplivre_core for iOS devices and Simulator

set -e

echo "🔨 Building Rust core library for iOS..."
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Project root
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CORE_DIR="$PROJECT_ROOT/core"
IOS_DIR="$PROJECT_ROOT/ios"
LIBRARIES_DIR="$IOS_DIR/Libraries"

echo -e "${BLUE}Project root:${NC} $PROJECT_ROOT"
echo -e "${BLUE}Core directory:${NC} $CORE_DIR"
echo -e "${BLUE}iOS directory:${NC} $IOS_DIR"
echo ""

# Work around older CMake minimums in bundled deps (e.g., audiopus)
export CMAKE_POLICY_VERSION_MINIMUM=3.5

# Align Rust link flags with iOS deployment target to avoid ld warnings/symbols
export IPHONEOS_DEPLOYMENT_TARGET=13.0
export CARGO_TARGET_AARCH64_APPLE_IOS_RUSTFLAGS="-C link-arg=-miphoneos-version-min=13.0"
export CARGO_TARGET_AARCH64_APPLE_IOS_SIM_RUSTFLAGS="-C link-arg=-mios-simulator-version-min=13.0"
export CARGO_TARGET_X86_64_APPLE_IOS_RUSTFLAGS="-C link-arg=-mios-simulator-version-min=13.0"

# Create Libraries directory
mkdir -p "$LIBRARIES_DIR"

# Build for iOS device (ARM64)
echo -e "${GREEN}Building for iOS device (aarch64-apple-ios)...${NC}"
cd "$CORE_DIR"
cargo build --release --target aarch64-apple-ios --features voip -p zaplivre-core
echo ""

# Build for iOS Simulator (ARM64 - Apple Silicon)
echo -e "${GREEN}Building for iOS Simulator ARM64 (aarch64-apple-ios-sim)...${NC}"
cargo build --release --target aarch64-apple-ios-sim --features voip -p zaplivre-core
echo ""

# Build for iOS Simulator (x86_64 - Intel)
echo -e "${GREEN}Building for iOS Simulator x86_64 (x86_64-apple-ios)...${NC}"
cargo build --release --target x86_64-apple-ios --features voip -p zaplivre-core
echo ""

# Copy iOS device library
echo -e "${GREEN}Copying iOS device library...${NC}"
cp "$PROJECT_ROOT/target/aarch64-apple-ios/release/libzaplivre_core.a" \
   "$LIBRARIES_DIR/libzaplivre_core_ios.a"
echo -e "  ✅ libzaplivre_core_ios.a"

# Create universal simulator binary (ARM64 + x86_64)
echo -e "${GREEN}Creating universal Simulator library...${NC}"
lipo -create \
  "$PROJECT_ROOT/target/aarch64-apple-ios-sim/release/libzaplivre_core.a" \
  "$PROJECT_ROOT/target/x86_64-apple-ios/release/libzaplivre_core.a" \
  -output "$LIBRARIES_DIR/libzaplivre_core_sim.a"
echo -e "  ✅ libzaplivre_core_sim.a (universal: ARM64 + x86_64)"
echo ""

# Show library sizes
echo -e "${GREEN}Library sizes:${NC}"
ls -lh "$LIBRARIES_DIR"
echo ""

# Verify architectures
echo -e "${GREEN}Verifying architectures:${NC}"
echo -e "${YELLOW}iOS device:${NC}"
lipo -info "$LIBRARIES_DIR/libzaplivre_core_ios.a"
echo -e "${YELLOW}Simulator:${NC}"
lipo -info "$LIBRARIES_DIR/libzaplivre_core_sim.a"
echo ""

echo -e "${GREEN}✅ Build complete!${NC}"
echo ""
echo -e "${BLUE}Next steps:${NC}"
echo "  1. Generate Swift bindings: cd ios && ./generate-bindings.sh"
echo "  2. Build Xcode project: xcodegen generate && xcodebuild -scheme ZapLivre build"
