# Repository Guidelines

## Project Structure & Module Organization
- `core/` is the Rust workspace (`zaplivre-core`) and contains core libraries, FFI, and most unit tests.
- `server/` hosts backend services (bootstrap, identity, push, store), each as its own Rust crate with local READMEs.
- `android/`, `ios/`, and `desktop/` contain the platform apps (Android/Kotlin, iOS/Swift, Tauri/React/TypeScript).
- `proto/` stores Protocol Buffers definitions used across services.
- `docs/` contains architecture and guide documents; `scripts/` has build helpers.

## Build, Test, and Development Commands
- `make setup` copies `.env.example` to `.env` and prefetches Rust deps.
- `make up` / `make down` start or stop the local backend stack via Docker Compose.
- `cd core && cargo build --release` builds the Rust core; `cargo test --workspace` runs core tests.
- `cd core && cargo clippy -- -D warnings` and `cargo fmt` enforce linting/formatting.
- `cd desktop && npm install && npm run tauri dev` runs the desktop app in dev mode.
- `cd android && ./gradlew assembleDebug` builds a debug APK (see `BUILD_AND_TEST.md` for VoIP flows).

## Coding Style & Naming Conventions
- Rust uses `cargo fmt` defaults (4-space indentation) with a 100‑char line limit.
- Naming: `snake_case` for functions/vars, `PascalCase` for structs/enums.
- Document public Rust APIs with `///` and prefer `Result<T>` for fallible functions.

## Testing Guidelines
- Unit tests live alongside modules; integration tests exist in `core/tests/` (e.g., `identity_integration`).
- Run integration tests with:
  - `cd core && cargo test --test identity_integration --features integration-tests`
- When touching backend services, verify the Docker stack (`make up`) and relevant crate tests.

## Commit & Pull Request Guidelines
- Use Conventional Commits: `type(scope): short description` (e.g., `feat(core): add key rotation`).
- Keep PRs focused (one feature per PR) and include:
  - Clear description, test notes, and any screenshots for UI changes.
  - Checklist items from `CONTRIBUTING.md` (fmt, clippy, tests).

## Configuration Tips
- Local env defaults live in `.env.example`; copy to `.env` before running services.
- Component-specific build steps are documented in `BUILD_AND_TEST.md` and platform READMEs.
