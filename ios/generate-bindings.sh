#!/bin/bash
# Generate Swift bindings from Rust core using UniFFI
# Requires: uniffi-bindgen 0.31.x (cargo install uniffi --version '^0.31' --features cli)

set -e

echo "🔨 Generating Swift bindings for ZapLivre Core..."
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# Directories
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CORE_DIR="$PROJECT_ROOT/core"
IOS_DIR="$PROJECT_ROOT/ios"
GENERATED_DIR="$IOS_DIR/ZapLivre/Generated"

# Files
UDL_FILE="$CORE_DIR/src/zaplivre.udl"

echo -e "${BLUE}Project root:${NC} $PROJECT_ROOT"
echo -e "${BLUE}UDL file:${NC} $UDL_FILE"
echo -e "${BLUE}Output directory:${NC} $GENERATED_DIR"
echo ""

# Check if UDL exists
if [ ! -f "$UDL_FILE" ]; then
    echo -e "${RED}❌ Error: UDL file not found at $UDL_FILE${NC}"
    exit 1
fi

# Create output directory
mkdir -p "$GENERATED_DIR"

# Check if uniffi-bindgen is available (prefer cargo version)
if ! command -v uniffi-bindgen &> /dev/null; then
    echo -e "${RED}❌ Error: uniffi-bindgen not found${NC}"
    echo ""
    echo "Install with:"
    echo "  cargo install uniffi --version '^0.31' --features cli"
    exit 1
fi

# Verify uniffi-bindgen version matches Rust uniffi version (0.31.x)
BINDGEN_VERSION=$(uniffi-bindgen --version | grep -oE '[0-9]+\.[0-9]+')
if [ "$BINDGEN_VERSION" != "0.31" ]; then
    echo -e "${YELLOW}⚠️  Warning: uniffi-bindgen version mismatch${NC}"
    echo "  Expected: 0.31.x"
    echo "  Found: $BINDGEN_VERSION"
    echo ""
    echo "To fix, reinstall uniffi-bindgen:"
    echo "  cargo install uniffi --version '^0.31' --features cli --force"
fi

# Generate bindings (uniffi 0.31+ syntax)
echo -e "${GREEN}Generating Swift bindings with uniffi-bindgen $(uniffi-bindgen --version)...${NC}"
uniffi-bindgen generate "$UDL_FILE" \
  --language swift \
  --out-dir "$GENERATED_DIR"

# Check if generation was successful
if [ $? -eq 0 ]; then
    echo ""
    echo -e "${GREEN}✅ Swift bindings generated successfully!${NC}"
    echo ""
    echo -e "${BLUE}Generated files:${NC}"
    ls -lh "$GENERATED_DIR"
    echo ""
    echo -e "${BLUE}Next steps:${NC}"
    echo "  1. Build iOS app: xcodegen generate && xcodebuild -scheme ZapLivre build"
    echo "  2. Run on simulator: open ios/ZapLivre.xcodeproj"
else
    echo ""
    echo -e "${RED}❌ Failed to generate Swift bindings${NC}"
    exit 1
fi
