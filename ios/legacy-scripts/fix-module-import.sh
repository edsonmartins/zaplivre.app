#!/bin/bash

# Fix "No such module 'mepassa'" error in Xcode

set -e

cd "$(dirname "$0")"

echo "=========================================="
echo "🔧 Fixing 'No such module mepassa' error"
echo "=========================================="
echo ""

# 1. Check if Generated files exist
echo "1️⃣ Checking Generated files..."
if [ ! -f "MePassa/Generated/mepassa.swift" ]; then
    echo "❌ mepassa.swift not found!"
    echo "Running: ./generate-bindings.sh"
    ./generate-bindings.sh
else
    echo "✅ mepassa.swift exists"
fi

if [ ! -f "MePassa/Generated/mepassaFFI.h" ]; then
    echo "❌ mepassaFFI.h not found!"
    exit 1
else
    echo "✅ mepassaFFI.h exists"
fi

if [ ! -f "MePassa/Generated/mepassaFFI.modulemap" ]; then
    echo "❌ mepassaFFI.modulemap not found!"
    exit 1
else
    echo "✅ mepassaFFI.modulemap exists"
fi

echo ""

# 2. Regenerate Xcode project
echo "2️⃣ Regenerating Xcode project..."
xcodegen generate
echo "✅ Xcode project regenerated"
echo ""

# 3. Instructions
echo "=========================================="
echo "✅ Fix Applied! Now in Xcode:"
echo "=========================================="
echo ""
echo "If still getting 'No such module mepassa':"
echo ""
echo "1. Close Xcode completely (⌘Q)"
echo "2. Delete DerivedData:"
echo "   rm -rf ~/Library/Developer/Xcode/DerivedData/MePassa-*"
echo ""
echo "3. Reopen project:"
echo "   open MePassa.xcodeproj"
echo ""
echo "4. Clean Build Folder:"
echo "   Product → Clean Build Folder (⇧⌘K)"
echo ""
echo "5. Build again (⌘B)"
echo ""
echo "Alternative: Instead of 'import mepassa', try:"
echo "  - Remove all 'import mepassa' lines"
echo "  - Rebuild - Swift files should still compile"
echo "  - The bridging header provides access to FFI"
echo ""
