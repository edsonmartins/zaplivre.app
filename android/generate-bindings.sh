#!/bin/bash
# Generate Kotlin bindings from Rust core using UniFFI
# Requires: uniffi-bindgen (pip install uniffi-bindgen==0.28.3)

set -e

echo "🔨 Generating Kotlin bindings for ZapLivre Core..."
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
ANDROID_DIR="$PROJECT_ROOT/android"
GENERATED_DIR="$ANDROID_DIR/app/src/main/kotlin"

# Files
UDL_FILE="$CORE_DIR/src/zaplivre.udl"

# Detect platform and use appropriate library extension
if [[ "$OSTYPE" == "darwin"* ]]; then
    LIB_FILE="$PROJECT_ROOT/target/release/libzaplivre_core.dylib"
else
    LIB_FILE="$PROJECT_ROOT/target/release/libzaplivre_core.so"
fi

echo -e "${BLUE}Project root:${NC} $PROJECT_ROOT"
echo -e "${BLUE}UDL file:${NC} $UDL_FILE"
echo -e "${BLUE}Library:${NC} $LIB_FILE"
echo -e "${BLUE}Output directory:${NC} $GENERATED_DIR"
echo ""

# Check if UDL exists
if [ ! -f "$UDL_FILE" ]; then
    echo -e "${RED}❌ Error: UDL file not found at $UDL_FILE${NC}"
    exit 1
fi

# Check if library exists (for x86_64)
if [ ! -f "$LIB_FILE" ]; then
    echo -e "${YELLOW}⚠️  Library not found, building for host (x86_64)...${NC}"
    cd "$CORE_DIR"
    cargo build --release --features voip -p zaplivre-core
    echo ""
fi

# Create output directory
mkdir -p "$GENERATED_DIR"

# Activate virtual environment if it exists
if [ -d "$ANDROID_DIR/venv" ]; then
    echo -e "${GREEN}Activating Python virtual environment...${NC}"
    source "$ANDROID_DIR/venv/bin/activate"
fi

# Check if uniffi-bindgen is available
if ! command -v uniffi-bindgen &> /dev/null; then
    echo -e "${RED}❌ Error: uniffi-bindgen not found${NC}"
    echo ""
    echo "Install with:"
    echo "  pip install uniffi-bindgen==0.28.3"
    echo ""
    echo "Or create a virtual environment:"
    echo "  cd android && python3 -m venv venv"
    echo "  source venv/bin/activate"
    echo "  pip install uniffi-bindgen==0.28.3"
    exit 1
fi

# Generate bindings
echo -e "${GREEN}Generating Kotlin bindings...${NC}"
uniffi-bindgen generate "$UDL_FILE" \
  --language kotlin \
  --out-dir "$GENERATED_DIR" \
  --lib-file "$LIB_FILE"

# Check if generation was successful
if [ $? -eq 0 ]; then
    echo ""
    echo -e "${GREEN}✅ Kotlin bindings generated successfully!${NC}"
    echo ""
    echo -e "${BLUE}Generated files:${NC}"
    ls -lh "$GENERATED_DIR"
    echo ""
    echo -e "${BLUE}Next steps:${NC}"
    echo "  1. Build native libraries for Android targets:"
    echo "     cd android && ./build-native.sh"
    echo "  2. Build Android app:"
    echo "     cd android && ./gradlew assembleDebug"
    echo "  3. Install on device/emulator:"
    echo "     adb install app/build/outputs/apk/debug/app-debug.apk"
else
    echo ""
    echo -e "${RED}❌ Failed to generate Kotlin bindings${NC}"
    exit 1
fi
