# Build DMG (macOS)

This helper wraps the Tauri DMG bundler script, skips Finder AppleScript, and works around common macOS `hdiutil` issues.

## Prerequisites

- Run a production build first:

```bash
cd desktop
npm run tauri:build
```

## Generate DMG

From the repo root:

```bash
./scripts/build-dmg.sh
```

The script reads the version from `desktop/src-tauri/tauri.conf.json` and falls back to `desktop/package.json`.

## Troubleshooting

- **`hdiutil create failed - Dispositivo não configurado`**
  - Run the script with elevated permissions:

```bash
sudo ./scripts/build-dmg.sh
```

- **`Failed to create app icon` / `No matching IconType`**
  - Regenerate icons and rebuild:

```bash
cd desktop
npm run tauri icon -- /path/to/icon-1024.png
npm run tauri:build
```
