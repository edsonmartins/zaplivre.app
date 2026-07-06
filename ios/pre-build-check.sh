#!/bin/bash

# ZapLivre iOS - Pre-Build Verification Script
# Verifica se tudo está pronto para rodar no iPhone

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "=========================================="
echo "🔍 ZapLivre iOS - Pre-Build Check"
echo "=========================================="
echo ""

# Track overall status
ALL_CHECKS_PASSED=true

# Function to print check result
check_status() {
    local name="$1"
    local status="$2"
    local message="$3"

    if [ "$status" = "OK" ]; then
        echo -e "${GREEN}✅ $name${NC}"
        [ -n "$message" ] && echo -e "   ${BLUE}ℹ️  $message${NC}"
    elif [ "$status" = "WARN" ]; then
        echo -e "${YELLOW}⚠️  $name${NC}"
        [ -n "$message" ] && echo -e "   ${YELLOW}→ $message${NC}"
    else
        echo -e "${RED}❌ $name${NC}"
        [ -n "$message" ] && echo -e "   ${RED}→ $message${NC}"
        ALL_CHECKS_PASSED=false
    fi
}

echo "📋 Checking System Requirements..."
echo ""

# 1. Check macOS version
MACOS_VERSION=$(sw_vers -productVersion)
check_status "macOS Version: $MACOS_VERSION" "OK"

# 2. Check Xcode
if command -v xcodebuild &> /dev/null; then
    XCODE_VERSION=$(xcodebuild -version | head -n 1)
    XCODE_BUILD=$(xcodebuild -version | tail -n 1)
    check_status "Xcode: $XCODE_VERSION ($XCODE_BUILD)" "OK"
else
    check_status "Xcode" "FAIL" "Xcode não encontrado. Instale via App Store."
fi

# 3. Check Rust
if command -v rustc &> /dev/null; then
    RUST_VERSION=$(rustc --version)
    check_status "Rust: $RUST_VERSION" "OK"
else
    check_status "Rust" "FAIL" "Rust não encontrado. Instale com: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
fi

# 4. Check iOS targets
echo ""
echo "📋 Checking Rust iOS Targets..."
echo ""

TARGETS_MISSING=false

if rustup target list | grep -q "aarch64-apple-ios (installed)"; then
    check_status "aarch64-apple-ios (iPhone ARM64)" "OK"
else
    check_status "aarch64-apple-ios (iPhone ARM64)" "FAIL" "Run: rustup target add aarch64-apple-ios"
    TARGETS_MISSING=true
fi

if rustup target list | grep -q "aarch64-apple-ios-sim (installed)"; then
    check_status "aarch64-apple-ios-sim (Simulator Apple Silicon)" "OK"
else
    check_status "aarch64-apple-ios-sim (Simulator Apple Silicon)" "WARN" "Run: rustup target add aarch64-apple-ios-sim"
fi

if rustup target list | grep -q "x86_64-apple-ios (installed)"; then
    check_status "x86_64-apple-ios (Simulator Intel)" "OK"
else
    check_status "x86_64-apple-ios (Simulator Intel)" "WARN" "Run: rustup target add x86_64-apple-ios"
fi

# 5. Check xcodegen
echo ""
echo "📋 Checking Build Tools..."
echo ""

if command -v xcodegen &> /dev/null; then
    XCODEGEN_VERSION=$(xcodegen --version)
    check_status "xcodegen: $XCODEGEN_VERSION" "OK"
else
    check_status "xcodegen" "FAIL" "Install with: brew install xcodegen"
fi

# 6. Check uniffi-bindgen
if [ -f "$SCRIPT_DIR/venv/bin/uniffi-bindgen" ]; then
    UNIFFI_VERSION=$("$SCRIPT_DIR/venv/bin/uniffi-bindgen" --version 2>/dev/null || echo "unknown")
    check_status "uniffi-bindgen (venv): $UNIFFI_VERSION" "OK"
elif command -v uniffi-bindgen &> /dev/null; then
    UNIFFI_VERSION=$(uniffi-bindgen --version 2>/dev/null || echo "unknown")
    check_status "uniffi-bindgen (global): $UNIFFI_VERSION" "OK"
else
    check_status "uniffi-bindgen" "FAIL" "Install with: cargo install uniffi --version ^0.31 --features cli"
fi

# 7. Check Rust libraries
echo ""
echo "📦 Checking Compiled Libraries..."
echo ""

IOS_LIB="$SCRIPT_DIR/Libraries/libzaplivre_core_ios.a"
SIM_LIB="$SCRIPT_DIR/Libraries/libzaplivre_core_sim.a"

if [ -f "$IOS_LIB" ]; then
    IOS_SIZE=$(du -h "$IOS_LIB" | cut -f1)
    IOS_DATE=$(stat -f "%Sm" -t "%Y-%m-%d %H:%M" "$IOS_LIB")
    check_status "libzaplivre_core_ios.a ($IOS_SIZE)" "OK" "Built: $IOS_DATE"
else
    check_status "libzaplivre_core_ios.a" "FAIL" "Run: ./ios/build-rust.sh"
fi

if [ -f "$SIM_LIB" ]; then
    SIM_SIZE=$(du -h "$SIM_LIB" | cut -f1)
    SIM_DATE=$(stat -f "%Sm" -t "%Y-%m-%d %H:%M" "$SIM_LIB")
    check_status "libzaplivre_core_sim.a ($SIM_SIZE)" "OK" "Built: $SIM_DATE"
else
    check_status "libzaplivre_core_sim.a" "WARN" "Run: ./ios/build-rust.sh (needed for Simulator)"
fi

# 8. Check Swift bindings
echo ""
echo "📦 Checking Swift Bindings..."
echo ""

SWIFT_BINDING="$SCRIPT_DIR/ZapLivre/Generated/zaplivre.swift"
FFI_HEADER="$SCRIPT_DIR/ZapLivre/Generated/zaplivreFFI.h"
MODULEMAP="$SCRIPT_DIR/ZapLivre/Generated/zaplivreFFI.modulemap"

if [ -f "$SWIFT_BINDING" ]; then
    SWIFT_SIZE=$(du -h "$SWIFT_BINDING" | cut -f1)
    SWIFT_DATE=$(stat -f "%Sm" -t "%Y-%m-%d %H:%M" "$SWIFT_BINDING")
    check_status "zaplivre.swift ($SWIFT_SIZE)" "OK" "Generated: $SWIFT_DATE"
else
    check_status "zaplivre.swift" "FAIL" "Run: ./ios/generate-bindings.sh"
fi

if [ -f "$FFI_HEADER" ]; then
    FFI_SIZE=$(du -h "$FFI_HEADER" | cut -f1)
    check_status "zaplivreFFI.h ($FFI_SIZE)" "OK"
else
    check_status "zaplivreFFI.h" "FAIL" "Run: ./ios/generate-bindings.sh"
fi

if [ -f "$MODULEMAP" ]; then
    check_status "zaplivreFFI.modulemap" "OK"
else
    check_status "zaplivreFFI.modulemap" "FAIL" "Run: ./ios/generate-bindings.sh"
fi

# 9. Check Xcode project
echo ""
echo "📦 Checking Xcode Project..."
echo ""

XCODE_PROJECT="$SCRIPT_DIR/ZapLivre.xcodeproj"
if [ -d "$XCODE_PROJECT" ]; then
    check_status "ZapLivre.xcodeproj" "OK"
else
    check_status "ZapLivre.xcodeproj" "FAIL" "Run: cd ios && xcodegen generate"
fi

# 10. Check for connected devices
echo ""
echo "📱 Checking Connected Devices..."
echo ""

DEVICES=$(xcrun xctrace list devices 2>&1 | grep "iPhone" | grep -v "Simulator" | head -n 5)
if [ -n "$DEVICES" ]; then
    echo -e "${GREEN}✅ iPhone(s) Connected:${NC}"
    echo "$DEVICES" | while IFS= read -r line; do
        echo -e "   ${BLUE}→ $line${NC}"
    done
else
    check_status "Connected iPhone" "WARN" "Nenhum iPhone conectado. Conecte via USB para testar em device físico."
fi

# 11. Check Apple ID in Xcode
echo ""
echo "👤 Checking Xcode Accounts..."
echo ""

ACCOUNTS=$(defaults read com.apple.dt.Xcode IDEProvisioningTeams 2>/dev/null || echo "")
if [ -n "$ACCOUNTS" ]; then
    check_status "Apple ID configured in Xcode" "OK" "Verifique team em: Xcode > Preferences > Accounts"
else
    check_status "Apple ID in Xcode" "WARN" "Adicione Apple ID em: Xcode > Preferences > Accounts > +"
fi

# Summary
echo ""
echo "=========================================="
echo "📊 Summary"
echo "=========================================="
echo ""

if [ "$ALL_CHECKS_PASSED" = true ]; then
    echo -e "${GREEN}✅ All critical checks passed!${NC}"
    echo ""
    echo "🚀 Ready to build for iPhone:"
    echo ""
    echo -e "${BLUE}Option 1 - Open in Xcode:${NC}"
    echo "  open ios/ZapLivre.xcodeproj"
    echo "  (Then: Connect iPhone, select device, press ▶️)"
    echo ""
    echo -e "${BLUE}Option 2 - Command line build:${NC}"
    echo "  ./ios/build-all.sh --build"
    echo ""
    echo -e "${YELLOW}⚠️  First time on device?${NC}"
    echo "  1. Connect iPhone via USB"
    echo "  2. Unlock iPhone and 'Trust This Computer'"
    echo "  3. In Xcode: Select your Team in Signing & Capabilities"
    echo "  4. Change Bundle ID if needed (e.g., com.yourname.zaplivre)"
    echo "  5. After install: iPhone Settings > General > VPN & Device Management > Trust Developer"
else
    echo -e "${RED}❌ Some checks failed!${NC}"
    echo ""
    echo -e "${YELLOW}🔧 Quick fix - Run all build steps:${NC}"
    echo "  ./ios/build-all.sh"
    echo ""
    echo -e "${YELLOW}📖 Or fix individually:${NC}"

    if [ "$TARGETS_MISSING" = true ]; then
        echo "  rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios"
    fi

    if [ ! -f "$IOS_LIB" ] || [ ! -f "$SIM_LIB" ]; then
        echo "  ./ios/build-rust.sh"
    fi

    if [ ! -f "$SWIFT_BINDING" ]; then
        echo "  ./ios/generate-bindings.sh"
    fi

    if [ ! -d "$XCODE_PROJECT" ]; then
        echo "  cd ios && xcodegen generate"
    fi
fi

echo ""
echo "=========================================="

# Return appropriate exit code
if [ "$ALL_CHECKS_PASSED" = true ]; then
    exit 0
else
    exit 1
fi
