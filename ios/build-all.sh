#!/bin/bash
# Complete build pipeline for ZapLivre iOS app
# 1. Build Rust core for iOS targets
# 2. Generate Swift bindings
# 3. Generate Xcode project
# 4. Build iOS app

set -e

echo "🚀 ZapLivre iOS - Complete Build Pipeline"
echo "=========================================="
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

IOS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Step 1: Build Rust core
echo -e "${BLUE}Step 1/4: Building Rust core libraries...${NC}"
"$IOS_DIR/build-rust.sh"
echo ""

# Step 2: Generate Swift bindings
echo -e "${BLUE}Step 2/4: Generating Swift bindings...${NC}"
"$IOS_DIR/generate-bindings.sh"
echo ""

# Step 3: Generate Xcode project
echo -e "${BLUE}Step 3/4: Generating Xcode project...${NC}"
cd "$IOS_DIR"
xcodegen generate
echo -e "${GREEN}✅ Xcode project generated${NC}"
echo ""

# Step 4: Build iOS app (optional)
if [ "$1" == "--build" ]; then
    echo -e "${BLUE}Step 4/4: Building iOS app...${NC}"
    xcodebuild -project ZapLivre.xcodeproj \
               -scheme ZapLivre \
               -sdk iphonesimulator \
               -destination 'platform=iOS Simulator,name=iPhone 16' \
               build
    echo ""
    echo -e "${GREEN}✅ iOS app built successfully!${NC}"
else
    echo -e "${YELLOW}Step 4/4: Skipped (run with --build to compile Xcode project)${NC}"
fi

echo ""
echo -e "${GREEN}=========================================="
echo -e "✅ Build pipeline complete!"
echo -e "==========================================${NC}"
echo ""
echo -e "${BLUE}To open in Xcode:${NC}"
echo "  open ios/ZapLivre.xcodeproj"
echo ""
echo -e "${BLUE}To build and run:${NC}"
echo "  ./ios/build-all.sh --build"
