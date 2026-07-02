#!/bin/bash
# Script to generate Swift bindings from mepassa-core
# This uses UniFFI to generate Swift code from the UDL definition

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CORE_DIR="$PROJECT_ROOT/core"
OUTPUT_DIR="$SCRIPT_DIR/MePassa/Generated"

echo "🔨 Generating Swift bindings for MePassa Core..."

# 1. Build the core library first
echo "📦 Building mepassa-core..."
cd "$CORE_DIR"
cargo build --release --lib

# 2. Find the library path (workspace root target directory)
# Note: Cargo workspace builds go to workspace root, not package directory
if [[ "$OSTYPE" == "darwin"* ]]; then
    LIB_PATH="$PROJECT_ROOT/target/release/libmepassa_core.dylib"
else
    echo "❌ Error: This script is currently only for macOS/iOS"
    exit 1
fi

if [ ! -f "$LIB_PATH" ]; then
    echo "❌ Error: Library not found at $LIB_PATH"
    exit 1
fi

echo "✅ Library built at: $LIB_PATH"

# 3. Create output directory
mkdir -p "$OUTPUT_DIR"

# 4. Generate Swift bindings using uniffi-bindgen
echo "🔧 Generating Swift bindings..."
cd "$PROJECT_ROOT"

# Install uniffi-bindgen if not already installed
if ! command -v uniffi-bindgen &> /dev/null; then
    echo "📥 Installing uniffi-bindgen..."
    cargo install uniffi-bindgen --version 0.31.0
fi

# Generate Swift bindings
uniffi-bindgen generate \
    --library "$LIB_PATH" \
    --language swift \
    --out-dir "$OUTPUT_DIR"

if [ $? -eq 0 ]; then
    echo "✅ Swift bindings generated in: $OUTPUT_DIR"
    echo ""
    echo "📝 Generated files:"
    ls -lh "$OUTPUT_DIR"
    echo ""
    echo "🎯 Next steps:"
    echo "   1. Add generated files to Xcode project"
    echo "   2. Add libmepassa_core.dylib to 'Frameworks and Libraries'"
    echo "   3. Import mepassa in Swift: import mepassa"
else
    echo "❌ Error generating Swift bindings"
    exit 1
fi
