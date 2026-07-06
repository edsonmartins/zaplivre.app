#!/usr/bin/env bash
# ZapLivre Build Script
# Builds all components of the project

set -e  # Exit on error
set -u  # Exit on undefined variable

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check prerequisites
check_prerequisites() {
    print_info "Checking prerequisites..."

    if ! command_exists cargo; then
        print_error "Rust/Cargo not found. Install from: https://rustup.rs"
        exit 1
    fi

    print_info "✓ Rust $(rustc --version | cut -d' ' -f2)"
}

# Build Rust core
build_core() {
    print_info "Building Rust core library..."

    cd core

    # Check
    cargo check --workspace

    # Format
    cargo fmt --all -- --check || {
        print_warning "Code is not formatted. Run 'cargo fmt' to format."
    }

    # Lint
    cargo clippy --workspace --all-features -- -D warnings || {
        print_warning "Clippy warnings found. Please fix them."
    }

    # Build
    cargo build --workspace --release

    cd ..

    print_info "✓ Core library built successfully"
}

# Build Android
build_android() {
    print_info "Building Android app..."

    if ! command_exists cargo-ndk; then
        print_warning "cargo-ndk not found. Installing..."
        cargo install cargo-ndk
    fi

    # Build Rust core for Android targets
    cd core
    cargo ndk \
        -t arm64-v8a \
        -t armeabi-v7a \
        -t x86_64 \
        -t x86 \
        -o ../android/app/src/main/jniLibs \
        build --release

    cd ../android

    # Build Android app
    ./gradlew build

    cd ..

    print_info "✓ Android app built successfully"
}

# Build iOS
build_ios() {
    print_info "Building iOS app..."

    if [[ "$OSTYPE" != "darwin"* ]]; then
        print_warning "iOS build only supported on macOS. Skipping..."
        return
    fi

    if ! command_exists cargo-lipo; then
        print_warning "cargo-lipo not found. Installing..."
        cargo install cargo-lipo
    fi

    # Build Rust core for iOS targets
    cd core
    cargo lipo --release --targets aarch64-apple-ios,x86_64-apple-ios

    cd ../ios

    # Build iOS app
    xcodebuild \
        -workspace ZapLivre.xcworkspace \
        -scheme ZapLivre \
        -sdk iphoneos \
        -configuration Release \
        CODE_SIGNING_ALLOWED=NO \
        build

    cd ..

    print_info "✓ iOS app built successfully"
}

# Build Desktop
build_desktop() {
    print_info "Building Desktop app..."

    if ! command_exists node; then
        print_error "Node.js not found. Install from: https://nodejs.org"
        exit 1
    fi

    cd desktop

    # Install dependencies
    npm ci

    # Build
    npm run tauri build

    cd ..

    print_info "✓ Desktop app built successfully"
}

# Build server components
build_server() {
    print_info "Building server components..."

    # Build bootstrap node
    cargo build --release --package zaplivre-bootstrap

    # Build message store
    cargo build --release --package zaplivre-store

    # Build push server
    cargo build --release --package zaplivre-push

    print_info "✓ Server components built successfully"
}

# Main function
main() {
    print_info "==================================="
    print_info "    ZapLivre Build Script"
    print_info "==================================="

    check_prerequisites

    # Parse arguments
    if [ $# -eq 0 ]; then
        # Build everything
        build_core
        build_android || print_warning "Android build failed"
        build_ios || print_warning "iOS build failed"
        build_desktop || print_warning "Desktop build failed"
        build_server
    else
        case "$1" in
            core)
                build_core
                ;;
            android)
                build_core
                build_android
                ;;
            ios)
                build_core
                build_ios
                ;;
            desktop)
                build_core
                build_desktop
                ;;
            server)
                build_core
                build_server
                ;;
            *)
                print_error "Unknown target: $1"
                echo "Usage: $0 [core|android|ios|desktop|server]"
                exit 1
                ;;
        esac
    fi

    print_info "==================================="
    print_info "    Build completed successfully!"
    print_info "==================================="
}

# Run main function
main "$@"
