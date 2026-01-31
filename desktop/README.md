# MePassa Desktop

Desktop application for MePassa built with Tauri 2.0 + React + TypeScript + TailwindCSS.

## 🚀 Features

- **Tauri 2.0:** Lightweight desktop app with Rust backend
- **React 18:** Modern UI with hooks and functional components
- **TypeScript:** Type-safe development
- **TailwindCSS:** Utility-first CSS framework
- **System Tray:** Minimize to tray icon
- **Desktop Notifications:** Native notification support
- **FFI Integration:** Direct calls to mepassa-core Rust library
- **Cross-Platform:** Windows, macOS, Linux

## 📋 Prerequisites

- **Node.js:** 18+ with npm
- **Rust:** 1.70+ (install via [rustup](https://rustup.rs/))
- **Tauri CLI:** `npm install -g @tauri-apps/cli`
- **System dependencies:**
  - **macOS:** Xcode Command Line Tools
  - **Linux:** `webkit2gtk`, `libayatana-appindicator3` (see [Tauri prerequisites](https://tauri.app/v2/guides/getting-started/prerequisites))
  - **Windows:** WebView2 (usually pre-installed on Windows 10/11)

## 🛠️ Development

### Install dependencies

```bash
cd desktop
npm install
```

### Run in development mode

```bash
npm run tauri:dev
```

This will:
1. Start Vite dev server (React frontend) on `http://localhost:5173`
2. Compile Rust backend (Tauri)
3. Launch desktop app with hot-reload

### Build for production

```bash
npm run tauri:build
```

Artifacts will be created in `src-tauri/target/release/bundle/`:
- **macOS:** `.dmg` and `.app`
- **Windows:** `.msi` and `.exe`
- **Linux:** `.AppImage` and `.deb`

You can also build the macOS DMG from the repo root:

```bash
make dmg
```

**macOS DMG note:** On some machines, `hdiutil create` fails with “Dispositivo não configurado” when running in restricted environments. If the `.app` builds but the `.dmg` fails, rerun the build with elevated permissions or run the DMG script manually. See the DMG troubleshooting section below.

```bash
# Option A: rebuild with elevated permissions
sudo npm run tauri:build

# Option B: run the DMG script directly
bash desktop/src-tauri/target/release/bundle/dmg/bundle_dmg.sh \
  desktop/src-tauri/target/release/bundle/dmg/MePassa_0.1.0_x64.dmg \
  desktop/src-tauri/target/release/bundle/macos/MePassa.app
```

Alternatively, use the helper script from the repo root (it reads the version from `tauri.conf.json` and falls back to `package.json`):

```bash
./scripts/build-dmg.sh
```

Example output:

```text
Building DMG for version 0.1.0
Source: /path/to/desktop/src-tauri/target/release/bundle/macos/MePassa.app
Output: /path/to/desktop/src-tauri/target/release/bundle/dmg/MePassa_0.1.0_x64.dmg
DMG created at: /path/to/desktop/src-tauri/target/release/bundle/dmg/MePassa_0.1.0_x64.dmg
```

### DMG troubleshooting (macOS)

- **`hdiutil create failed - Dispositivo não configurado`:** rerun with elevated permissions (`sudo npm run tauri:build`) or run `./scripts/build-dmg.sh` with elevated permissions.
- **`Failed to create app icon` / `No matching IconType`:** regenerate icons with `npm run tauri icon -- <path-to-1024.png>` and re-run the build.

For a step-by-step guide, see `scripts/build-dmg.md` from the repo root.

## 📁 Project Structure

```
desktop/
├── src/                    # React frontend
│   ├── main.tsx            # Entry point
│   ├── App.tsx             # Main router
│   ├── views/              # Views (screens)
│   │   ├── OnboardingView.tsx
│   │   ├── ConversationsView.tsx
│   │   └── ChatView.tsx
│   ├── components/         # Reusable components
│   └── styles/
│       └── index.css       # Tailwind + custom styles
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── main.rs         # Entry point, system tray
│   │   └── commands.rs     # Tauri commands (FFI calls)
│   ├── Cargo.toml          # Rust dependencies
│   └── tauri.conf.json     # Tauri configuration
├── package.json
├── vite.config.ts
├── tsconfig.json
└── tailwind.config.js
```

## 🔌 Tauri Commands (FFI → mepassa-core)

All commands in `src-tauri/src/commands.rs`:

| Command | Description |
|---------|-------------|
| `init_client(data_dir)` | Initialize MePassa client |
| `get_local_peer_id()` | Get local peer ID |
| `listen_on(multiaddr)` | Start listening on address |
| `connect_to_peer(peer_id, multiaddr)` | Connect to peer |
| `send_text_message(to_peer_id, content)` | Send text message |
| `get_conversation_messages(peer_id, limit, offset)` | Get messages |
| `list_conversations()` | List all conversations |
| `search_messages(query, limit)` | Search messages (FTS5) |
| `mark_conversation_read(peer_id)` | Mark as read |
| `get_connected_peers_count()` | Get connected peers |
| `bootstrap()` | Bootstrap DHT |

## 🌐 Bootstrap Peers (Production)

Atualmente os peers de bootstrap usados pelo desktop vêm do core (FFI). Para apontar
para seus bootstraps públicos, ajuste a lista em `core/src/ffi/client.rs` conforme
o exemplo em `core/FFI_IMPLEMENTATION.md`.

### Example usage (TypeScript)

```typescript
import { invoke } from '@tauri-apps/api/core'

// Initialize client
const peerId = await invoke<string>('init_client', {
  dataDir: '/home/user/.mepassa'
})

// Send message
await invoke('send_text_message', {
  toPeerId: 'QmXYZ...',
  content: 'Hello, world!'
})

// List conversations
const conversations = await invoke<Conversation[]>('list_conversations')
```

## 🎨 UI Components

### Views

- **OnboardingView:** First-run experience, displays peer ID
- **ConversationsView:** List of conversations, new chat button
- **ChatView:** Chat interface with message bubbles

### Styling

Using TailwindCSS utility classes with custom components:

```css
.btn-primary      /* Primary button (teal) */
.btn-secondary    /* Secondary button (gray) */
.input-base       /* Input field base style */
.message-sent     /* Sent message bubble (right, teal) */
.message-received /* Received message bubble (left, gray) */
```

## 🔔 System Tray

Implemented in `src-tauri/src/main.rs`:

- **Left Click:** Show/focus window
- **Right Click:** Context menu
  - Show: Show/focus window
  - Quit: Exit application

## 📦 Build Configuration

### Cargo.toml

```toml
[dependencies]
tauri = { version = "2.0", features = ["tray-icon", "notification"] }
mepassa-core = { path = "../../core" }  # Local dependency
```

### tauri.conf.json

- **Product Name:** MePassa
- **Bundle ID:** com.integralltech.mepassa
- **Frontend:** Vite dev server (port 5173)
- **Output:** `../dist`
- **Bundle Targets:** DMG (macOS), MSI (Windows), AppImage (Linux)

## 🐛 Troubleshooting

### "Failed to initialize MePassa" error

**Cause:** Client initialization failed (likely data directory issue)

**Solution:**
```bash
# Check data directory permissions
ls -la ~/.mepassa

# Try manual initialization
rm -rf ~/.mepassa  # CAUTION: Deletes all data
```

### Vite dev server not starting

**Cause:** Port 5173 already in use

**Solution:**
```bash
# Find process using port 5173
lsof -i :5173

# Kill it or change port in vite.config.ts
```

### Tauri build fails on Linux

**Cause:** Missing system dependencies

**Solution (Ubuntu/Debian):**
```bash
sudo apt update
sudo apt install libwebkit2gtk-4.0-dev \
    build-essential \
    curl \
    wget \
    file \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev
```

### System tray icon not showing

**Cause:** Icon files missing in `src-tauri/icons/`

**Solution:**
- Generate icons using [Tauri Icon Generator](https://tauri.app/v1/guides/features/icons)
- Or use placeholder PNGs (32x32, 128x128)

## 📊 Metrics

| Metric | Value |
|--------|-------|
| Frontend Bundle Size | ~500 KB (minified) |
| Backend Binary Size | ~15 MB (release) |
| Memory Usage | ~80 MB (idle) |
| Startup Time | ~1 second |

## 🚀 Next Steps

- [ ] Add desktop notifications for new messages
- [ ] Implement VoIP calling UI (FASE 12)
- [ ] Add settings screen
- [ ] Implement file sharing UI
- [ ] Add keyboard shortcuts
- [ ] Improve system tray menu

## 📚 Resources

- [Tauri 2.0 Docs](https://tauri.app/v2/guides/)
- [React Docs](https://react.dev/)
- [TailwindCSS Docs](https://tailwindcss.com/docs)
- [Vite Docs](https://vitejs.dev/)

---

**Version:** 0.1.0-alpha
**Last Updated:** 2025-01-20
