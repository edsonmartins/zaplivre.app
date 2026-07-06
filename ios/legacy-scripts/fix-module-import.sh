#!/bin/bash

# Fix "No such module 'zaplivre'" error in Xcode

set -e

cd "$(dirname "$0")"

echo "=========================================="
echo "🔧 Fixing 'No such module zaplivre' error"
echo "=========================================="
echo ""

# 1. Check if Generated files exist
echo "1️⃣ Checking Generated files..."
if [ ! -f "ZapLivre/Generated/zaplivre.swift" ]; then
    echo "❌ zaplivre.swift not found!"
    echo "Running: ./generate-bindings.sh"
    ./generate-bindings.sh
else
    echo "✅ zaplivre.swift exists"
fi

if [ ! -f "ZapLivre/Generated/zaplivreFFI.h" ]; then
    echo "❌ zaplivreFFI.h not found!"
    exit 1
else
    echo "✅ zaplivreFFI.h exists"
fi

if [ ! -f "ZapLivre/Generated/zaplivreFFI.modulemap" ]; then
    echo "❌ zaplivreFFI.modulemap not found!"
    exit 1
else
    echo "✅ zaplivreFFI.modulemap exists"
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
echo "If still getting 'No such module zaplivre':"
echo ""
echo "1. Close Xcode completely (⌘Q)"
echo "2. Delete DerivedData:"
echo "   rm -rf ~/Library/Developer/Xcode/DerivedData/ZapLivre-*"
echo ""
echo "3. Reopen project:"
echo "   open ZapLivre.xcodeproj"
echo ""
echo "4. Clean Build Folder:"
echo "   Product → Clean Build Folder (⇧⌘K)"
echo ""
echo "5. Build again (⌘B)"
echo ""
echo "Alternative: Instead of 'import zaplivre', try:"
echo "  - Remove all 'import zaplivre' lines"
echo "  - Rebuild - Swift files should still compile"
echo "  - The bridging header provides access to FFI"
echo ""
