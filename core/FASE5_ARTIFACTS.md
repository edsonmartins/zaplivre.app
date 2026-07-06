# FASE 5 - Artefatos Gerados

**Data:** 2025-01-20
**Status:** ✅ 100% COMPLETA

## Bindings Gerados

### Kotlin (Android)
- **Arquivo:** `target/bindings/uniffi/zaplivre/zaplivre.kt`
- **Tamanho:** 80 KB
- **Descrição:** Bindings Kotlin para integração com Android app
- **Package:** `uniffi.zaplivre`

### Swift (iOS)
- **Arquivo principal:** `target/bindings/zaplivre.swift`
- **Tamanho:** 47 KB
- **Header C:** `target/bindings/zaplivreFFI.h` (26 KB)
- **Module map:** `target/bindings/zaplivreFFI.modulemap` (130 B)
- **Descrição:** Bindings Swift para integração com iOS app

## Bibliotecas Nativas Compiladas

### Android

#### ARM64 (aarch64-linux-android) - Arquitetura Principal
- **Arquivo:** `target/aarch64-linux-android/release/libzaplivre_core.so`
- **Tamanho:** 6.3 MB
- **Tipo:** Shared library (.so)
- **Uso:** Smartphones Android modernos (ARM 64-bit)
- **Min API Level:** 33 (Android 13)
- **Destino:** `android/app/src/main/jniLibs/arm64-v8a/libzaplivre_core.so`

#### Outras Arquiteturas (opcional)
Para suporte completo, compile também:
```bash
# ARM 32-bit (dispositivos antigos)
cargo build --target armv7-linux-androideabi --release --lib

# x86_64 (emuladores)
cargo build --target x86_64-linux-android --release --lib

# x86 (emuladores antigos)
cargo build --target i686-linux-android --release --lib
```

### iOS

#### ARM64 - Dispositivos Reais (iPhone/iPad)
- **Arquivo:** `target/aarch64-apple-ios/release/libzaplivre_core.a`
- **Tamanho:** 96 MB
- **Tipo:** Static library (.a)
- **Uso:** iPhones e iPads com processadores A12+ (iPhone XS+)
- **Min iOS:** 13.0

#### ARM64 - Simulador (Apple Silicon Macs)
- **Arquivo:** `target/aarch64-apple-ios-sim/release/libzaplivre_core.a`
- **Tamanho:** 96 MB
- **Tipo:** Static library (.a)
- **Uso:** Simulador iOS em Macs com Apple Silicon (M1/M2/M3)

#### x86_64 - Simulador (Intel Macs)
- **Arquivo:** `target/x86_64-apple-ios/release/libzaplivre_core.a`
- **Tamanho:** 95 MB
- **Tipo:** Static library (.a)
- **Uso:** Simulador iOS em Macs com Intel

### Desktop (macOS)

#### Biblioteca nativa já disponível
- **Arquivo:** `target/debug/libzaplivre_core.dylib` (dev)
- **Arquivo:** `target/release/libzaplivre_core.dylib` (prod)
- **Uso:** Tauri desktop app (já compilada automaticamente)

## Configuração de Build

### Android - NDK
- **NDK versão:** 26.3.11579264
- **Toolchain:** `darwin-x86_64` (macOS Intel/Rosetta)
- **API Level:** 33 (Android 13)
- **Config:** `core/.cargo/config.toml`

### iOS - Xcode
- **Toolchain:** Xcode Command Line Tools
- **Targets instaladas:**
  - `aarch64-apple-ios` (device)
  - `aarch64-apple-ios-sim` (simulator ARM)
  - `x86_64-apple-ios` (simulator Intel)

## Dependências Importantes

### Mudanças no Cargo.toml
```toml
# Workspace (Cargo.toml raiz)
reqwest = { version = "0.11", default-features = false, features = ["json", "rustls-tls"] }

# Core (core/Cargo.toml)
[dev-dependencies]
uniffi = { workspace = true, features = ["bindgen"] }
uniffi_bindgen = "0.31"
camino = "1.1"
```

**Nota:** Mudamos de `native-tls` para `rustls-tls` para evitar dependência de OpenSSL no Android.

## Comandos de Build

### Gerar Bindings
```bash
cd core
cargo run --example generate_bindings
```

### Compilar para Android
```bash
# Configurar variáveis de ambiente
export CC_aarch64_linux_android="/path/to/ndk/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android33-clang"
export AR_aarch64_linux_android="/path/to/ndk/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ar"

# Build
cargo build --target aarch64-linux-android --release --lib
```

### Compilar para iOS
```bash
# Device (ARM64)
cargo build --target aarch64-apple-ios --release --lib

# Simulator (Apple Silicon)
cargo build --target aarch64-apple-ios-sim --release --lib

# Simulator (Intel)
cargo build --target x86_64-apple-ios --release --lib
```

## Próximos Passos (FASE 6)

### Android App
1. Copiar `libzaplivre_core.so` para `android/app/src/main/jniLibs/arm64-v8a/`
2. Integrar bindings Kotlin no projeto Android
3. Criar `ZapLivreService` (foreground service)
4. Implementar UI básica (Jetpack Compose)

### iOS App
1. Criar XCFramework combinando todas as arquiteturas:
```bash
xcodebuild -create-xcframework \
  -library target/aarch64-apple-ios/release/libzaplivre_core.a \
  -library target/aarch64-apple-ios-sim/release/libzaplivre_core.a \
  -library target/x86_64-apple-ios/release/libzaplivre_core.a \
  -output ZapLivreCore.xcframework
```
2. Integrar bindings Swift no projeto iOS
3. Implementar UI básica (SwiftUI)

### Desktop App
1. Integrar bindings no Tauri (já usa dylib automaticamente)
2. Implementar UI (React + TailwindCSS)

## Estrutura de Arquivos

```
core/
├── .cargo/
│   └── config.toml          # Configuração NDK Android
├── examples/
│   └── generate_bindings.rs # Script de geração de bindings
├── target/
│   ├── bindings/            # Bindings gerados
│   │   ├── uniffi/zaplivre/
│   │   │   └── zaplivre.kt   # Kotlin
│   │   ├── zaplivre.swift    # Swift
│   │   ├── zaplivreFFI.h     # Swift header
│   │   └── zaplivreFFI.modulemap
│   ├── aarch64-linux-android/release/
│   │   └── libzaplivre_core.so  # Android
│   ├── aarch64-apple-ios/release/
│   │   └── libzaplivre_core.a   # iOS device
│   ├── aarch64-apple-ios-sim/release/
│   │   └── libzaplivre_core.a   # iOS sim ARM
│   └── x86_64-apple-ios/release/
│       └── libzaplivre_core.a   # iOS sim Intel
└── FFI_IMPLEMENTATION.md    # Documentação técnica
```

## Testes

### Verificar Símbolos (Android)
```bash
nm -D target/aarch64-linux-android/release/libzaplivre_core.so | grep uniffi
```

### Verificar Símbolos (iOS)
```bash
nm target/aarch64-apple-ios/release/libzaplivre_core.a | grep uniffi
```

### Teste Básico (Kotlin)
```kotlin
import uniffi.zaplivre.*

val client = ZapLivreClient("/data/local/tmp/zaplivre")
println(client.localPeerId())
```

### Teste Básico (Swift)
```swift
import zaplivreFFI

let client = try ZapLivreClient(dataDir: "/tmp/zaplivre")
print(try client.localPeerId())
```

## Métricas de Compilação

| Target | Tempo | Tamanho | Tipo |
|--------|-------|---------|------|
| Android ARM64 | 2m 47s | 6.3 MB | .so |
| iOS ARM64 | 2m 51s | 96 MB | .a |
| iOS Sim ARM64 | 2m 52s | 96 MB | .a |
| iOS Sim x86_64 | 2m 51s | 95 MB | .a |

**Total de compilações:** 4
**Tempo total:** ~11 minutos
**Warnings:** 1 (dead_code em `data_dir` - pode ser ignorado)

---

**Gerado em:** 2025-01-20
**Por:** FASE 5 - FFI Implementation
**Versão:** zaplivre-core v0.1.0
