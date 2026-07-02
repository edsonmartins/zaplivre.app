#!/usr/bin/env python3
"""
Generate Swift bindings for mepassa-core using uniffi-bindgen
This script uses the uniffi Python package to generate bindings
"""

import subprocess
import sys
from pathlib import Path

def main():
    # Setup paths
    project_root = Path(__file__).parent.parent
    core_dir = project_root / "core"
    udl_file = core_dir / "src" / "mepassa.udl"
    lib_file = project_root / "target" / "release" / "libmepassa_core.dylib"
    out_dir = Path(__file__).parent / "MePassa" / "Generated"

    print("🔨 Generating Swift bindings for MePassa Core...")
    print(f"📄 UDL file: {udl_file}")
    print(f"📚 Library: {lib_file}")
    print(f"📂 Output: {out_dir}")

    # Check if library exists
    if not lib_file.exists():
        print(f"❌ Error: Library not found at {lib_file}")
        print("   Please build the library first:")
        print("   cd core && cargo build --release")
        sys.exit(1)

    # Check if UDL exists
    if not udl_file.exists():
        print(f"❌ Error: UDL file not found at {udl_file}")
        sys.exit(1)

    # Create output directory
    out_dir.mkdir(parents=True, exist_ok=True)

    # Try to import uniffi_bindgen Python module
    try:
        import uniffi_bindgen
        print("✅ Found uniffi_bindgen Python module")

        # Use Python API
        from uniffi_bindgen import generate_bindings
        generate_bindings(
            udl_file=str(udl_file),
            config_file_override=None,
            out_dir=str(out_dir),
            language="swift",
            library_file=str(lib_file),
            crate_name="mepassa"
        )

        print("✅ Swift bindings generated successfully!")

    except ImportError:
        print("⚠️  uniffi_bindgen Python module not found")
        print("   Trying cargo-based approach...")

        # Fallback: Try using cargo run with uniffi
        try:
            # Change to core directory
            import os
            os.chdir(core_dir)

            # Run uniffi-bindgen through cargo
            result = subprocess.run([
                "cargo", "run",
                "--bin", "uniffi-bindgen",
                "--features", "uniffi/cli",
                "--",
                "generate",
                "--library", str(lib_file),
                "--language", "swift",
                "--out-dir", str(out_dir)
            ], capture_output=True, text=True)

            if result.returncode == 0:
                print("✅ Swift bindings generated successfully!")
            else:
                print(f"❌ Error generating bindings:")
                print(result.stderr)
                sys.exit(1)

        except Exception as e:
            print(f"❌ Error: {e}")
            print("\nManual generation required:")
            print(f"1. Install uniffi-bindgen Python: pip install uniffi-bindgen")
            print(f"2. Or use the Rust example: cd core && cargo run --example generate_bindings")
            sys.exit(1)

    # List generated files
    if out_dir.exists():
        print("\n📝 Generated files:")
        for file in sorted(out_dir.glob("*")):
            print(f"   - {file.name}")

    print("\n🎯 Next steps:")
    print("   1. Add generated files to Xcode project")
    print("   2. Add libmepassa_core.a to 'Frameworks and Libraries'")
    print("   3. Import mepassa in Swift: import mepassa")

if __name__ == "__main__":
    main()
