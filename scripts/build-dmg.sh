#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DESKTOP_DIR="$ROOT_DIR/desktop"
TAURI_CONF="$DESKTOP_DIR/src-tauri/tauri.conf.json"
PACKAGE_JSON="$DESKTOP_DIR/package.json"
DMG_SCRIPT="$DESKTOP_DIR/src-tauri/target/release/bundle/dmg/bundle_dmg.sh"
APP_PATH="$DESKTOP_DIR/src-tauri/target/release/bundle/macos/ZapLivre.app"
ICON_PATH="$DESKTOP_DIR/src-tauri/icons/icon.icns"

read_version_from_tauri() {
  if [[ -f "$TAURI_CONF" ]]; then
    rg --no-line-number '"version"' "$TAURI_CONF" | head -n 1 | sed -E 's/.*"version"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/'
  fi
}

read_version_from_package() {
  if [[ -f "$PACKAGE_JSON" ]]; then
    rg --no-line-number '"version"' "$PACKAGE_JSON" | head -n 1 | sed -E 's/.*"version"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/'
  fi
}

VERSION=$(read_version_from_tauri)
if [[ -z "$VERSION" ]]; then
  VERSION=$(read_version_from_package)
fi

if [[ -z "$VERSION" ]]; then
  echo "Failed to read version from $TAURI_CONF or $PACKAGE_JSON" >&2
  exit 1
fi

DMG_PATH="$DESKTOP_DIR/src-tauri/target/release/bundle/dmg/ZapLivre_${VERSION}_x64.dmg"

if [[ ! -x "$DMG_SCRIPT" ]]; then
  echo "DMG script not found. Run 'npm run tauri:build' first." >&2
  exit 1
fi

if [[ ! -d "$APP_PATH" ]]; then
  echo "App bundle not found at $APP_PATH" >&2
  echo "Run 'npm run tauri:build' first." >&2
  exit 1
fi

if [[ ! -f "$ICON_PATH" ]]; then
  echo "Icon not found at $ICON_PATH" >&2
  exit 1
fi

STAGING_DIR="$(mktemp -d /tmp/zaplivre-dmg.XXXX)"
cleanup() {
  rm -rf "$STAGING_DIR"
}
trap cleanup EXIT

ditto -rsrc "$APP_PATH" "$STAGING_DIR/ZapLivre.app"

echo "Building DMG for version $VERSION"
echo "Source: $APP_PATH"
echo "Output: $DMG_PATH"
echo "Staging: $STAGING_DIR"

bash "$DMG_SCRIPT" \
  --skip-jenkins \
  --volname "ZapLivre" \
  --volicon "$ICON_PATH" \
  --icon "ZapLivre.app" 140 160 \
  --app-drop-link 420 160 \
  --window-size 600 400 \
  --icon-size 128 \
  "$DMG_PATH" \
  "$STAGING_DIR"

echo "DMG created at: $DMG_PATH"
