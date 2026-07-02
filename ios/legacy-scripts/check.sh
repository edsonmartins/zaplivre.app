#!/bin/bash
# Simplified iOS Pre-Build Check

echo "=========================================="
echo "MePassa iOS - Pre-Build Check"
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
[ -f Libraries/libmepassa_core_ios.a ] && echo "✓ libmepassa_core_ios.a ($(du -h Libraries/libmepassa_core_ios.a | cut -f1))" || echo "✗ libmepassa_core_ios.a MISSING"
[ -f Libraries/libmepassa_core_sim.a ] && echo "✓ libmepassa_core_sim.a ($(du -h Libraries/libmepassa_core_sim.a | cut -f1))" || echo "✗ libmepassa_core_sim.a MISSING"
echo ""

echo "5. Swift Bindings"
echo "----------------"
[ -f MePassa/Generated/mepassa.swift ] && echo "✓ mepassa.swift ($(du -h MePassa/Generated/mepassa.swift | cut -f1))" || echo "✗ mepassa.swift MISSING"
[ -f MePassa/Generated/mepassaFFI.h ] && echo "✓ mepassaFFI.h" || echo "✗ mepassaFFI.h MISSING"
[ -f MePassa/Generated/mepassaFFI.modulemap ] && echo "✓ mepassaFFI.modulemap" || echo "✗ mepassaFFI.modulemap MISSING"
echo ""

echo "6. Xcode Project"
echo "----------------"
[ -d MePassa.xcodeproj ] && echo "✓ MePassa.xcodeproj" || echo "✗ MePassa.xcodeproj MISSING"
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
echo "  open MePassa.xcodeproj"
echo ""
echo "Add iOS targets:"
echo "  rustup target add aarch64-apple-ios aarch64-apple-ios-sim"
echo ""
