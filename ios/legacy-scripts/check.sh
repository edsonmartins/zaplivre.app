#!/bin/bash
# Simplified iOS Pre-Build Check

echo "=========================================="
echo "ZapLivre iOS - Pre-Build Check"
echo "=========================================="
echo ""

cd "$(dirname "$0")"

echo "1. System Checks"
echo "----------------"
echo "✓ macOS: $(sw_vers -productVersion)"
echo "✓ Xcode: $(xcodebuild -version 2>/dev/null | head -1 || echo 'NOT FOUND')"
echo "✓ Rust: $(rustc --version 2>/dev/null || echo 'NOT FOUND')"
echo ""

echo "2. Rust Targets"
echo "----------------"
rustup target list | grep apple-ios | grep installed || echo "⚠ Missing iOS targets"
echo ""

echo "3. Build Tools"
echo "----------------"
command -v xcodegen >/dev/null && echo "✓ xcodegen: $(xcodegen --version)" || echo "✗ xcodegen NOT FOUND"
[ -f venv/bin/uniffi-bindgen ] && echo "✓ uniffi-bindgen (venv)" || echo "⚠ uniffi-bindgen not in venv"
echo ""

echo "4. Compiled Libraries"
echo "----------------"
[ -f Libraries/libzaplivre_core_ios.a ] && echo "✓ libzaplivre_core_ios.a ($(du -h Libraries/libzaplivre_core_ios.a | cut -f1))" || echo "✗ libzaplivre_core_ios.a MISSING"
[ -f Libraries/libzaplivre_core_sim.a ] && echo "✓ libzaplivre_core_sim.a ($(du -h Libraries/libzaplivre_core_sim.a | cut -f1))" || echo "✗ libzaplivre_core_sim.a MISSING"
echo ""

echo "5. Swift Bindings"
echo "----------------"
[ -f ZapLivre/Generated/zaplivre.swift ] && echo "✓ zaplivre.swift ($(du -h ZapLivre/Generated/zaplivre.swift | cut -f1))" || echo "✗ zaplivre.swift MISSING"
[ -f ZapLivre/Generated/zaplivreFFI.h ] && echo "✓ zaplivreFFI.h" || echo "✗ zaplivreFFI.h MISSING"
[ -f ZapLivre/Generated/zaplivreFFI.modulemap ] && echo "✓ zaplivreFFI.modulemap" || echo "✗ zaplivreFFI.modulemap MISSING"
echo ""

echo "6. Xcode Project"
echo "----------------"
[ -d ZapLivre.xcodeproj ] && echo "✓ ZapLivre.xcodeproj" || echo "✗ ZapLivre.xcodeproj MISSING"
echo ""

echo "7. Connected Devices"
echo "----------------"
xcrun xctrace list devices 2>&1 | grep "iPhone" | grep -v "Simulator" | head -3 || echo "⚠ No iPhone connected"
echo ""

echo "=========================================="
echo "Quick Actions:"
echo "=========================================="
echo ""
echo "Build everything:"
echo "  ./build-all.sh"
echo ""
echo "Open in Xcode:"
echo "  open ZapLivre.xcodeproj"
echo ""
echo "Add iOS targets:"
echo "  rustup target add aarch64-apple-ios aarch64-apple-ios-sim"
echo ""
