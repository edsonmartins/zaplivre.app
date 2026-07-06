# ZapLivre Android App

App Android nativo com Jetpack Compose para a plataforma ZapLivre.

## 📋 Requisitos

- **Android Studio** Hedgehog (2023.1.1) ou superior
- **JDK** 17
- **Android SDK** API 34
- **Gradle** 8.5+
- **Dispositivo/Emulador** Android 8.0+ (API 26)

## 🏗️ Arquitetura

```
app/src/main/
├── kotlin/
│   ├── com/zaplivre/
│   │   ├── ZapLivreApplication.kt       # Application class
│   │   ├── MainActivity.kt             # Entry point
│   │   ├── core/
│   │   │   └── ZapLivreClientWrapper.kt # Wrapper do UniFFI client
│   │   ├── service/
│   │   │   └── ZapLivreService.kt       # Foreground service P2P
│   │   └── ui/
│   │       ├── theme/                  # Material3 theme
│   │       ├── navigation/             # Navigation Compose
│   │       └── screens/
│   │           ├── onboarding/         # Primeira tela
│   │           ├── conversations/      # Lista de conversas
│   │           └── chat/               # Chat individual
│   └── uniffi/zaplivre/
│       └── zaplivre.kt                  # Bindings gerados (UniFFI)
├── jniLibs/
│   └── arm64-v8a/
│       └── libzaplivre_core.so          # Biblioteca nativa
└── AndroidManifest.xml
```

## 🚀 Como Compilar

### 1. Preparar biblioteca nativa

A biblioteca `.so` já deve estar em `jniLibs/arm64-v8a/`. Se não estiver:

```bash
# No diretório raiz do projeto
cd core
cargo build --target aarch64-linux-android --release --lib

# Copiar para Android
cp target/aarch64-linux-android/release/libzaplivre_core.so \
   ../android/app/src/main/jniLibs/arm64-v8a/
```

### 2. Abrir no Android Studio

```bash
cd android
# Abrir no Android Studio ou via linha de comando:
open -a "Android Studio" .
```

### 3. Sync Gradle

Android Studio automaticamente fará sync do Gradle. Se não, clique em:
`File > Sync Project with Gradle Files`

### 4. Compilar

**Via Android Studio:**
- `Build > Make Project` (⌘F9 / Ctrl+F9)
- Ou clique no botão de play (Run)

**Via linha de comando:**
```bash
./gradlew assembleDebug      # Build debug
./gradlew assembleRelease    # Build release (requer signing)
./gradlew installDebug       # Instala no dispositivo conectado
```

## 📱 Como Executar

### No Emulador

1. Criar AVD (Android Virtual Device) no Android Studio
2. API 26+ com ARM64 (ou x86_64 se compilou para essa arquitetura)
3. Clicar em Run (▶️)

### No Dispositivo Real

1. Habilitar Developer Options no dispositivo
2. Habilitar USB Debugging
3. Conectar via USB
4. Autorizar o computador no dispositivo
5. Clicar em Run (▶️) e selecionar o dispositivo

## 🔧 Dependências Principais

| Dependência | Versão | Propósito |
|-------------|--------|-----------|
| Kotlin | 1.9.21 | Linguagem |
| Compose BOM | 2023.10.01 | UI Framework |
| Material3 | Latest | Design System |
| Navigation Compose | 2.7.6 | Navegação |
| Coroutines | 1.7.3 | Async/concurrency |
| JNA | 5.14.0 | UniFFI requirement |

## 📝 Fluxo do App

### 1. Primeira Execução (Onboarding)
- Gera keypair Ed25519
- Cria diretório de dados local
- Inicializa SQLite database
- Exibe Peer ID gerado

### 2. Uso Normal
- Inicia `ZapLivreService` (foreground)
- Conecta a bootstrap nodes
- Lista conversas existentes
- Permite enviar/receber mensagens P2P

## 🌐 Bootstrap Peers (Production)

Os peers de bootstrap usados pelo Android vêm do core (FFI). Para apontar para
seus bootstraps públicos, ajuste a lista em `core/src/ffi/client.rs` conforme
o exemplo em `core/FFI_IMPLEMENTATION.md`.

## 🌐 Configuração de produção (URLs)

Para builds de release, defina as URLs em `android/gradle.properties`:

```
MESSAGE_STORE_URL=https://store.associahub.com.br
PUSH_SERVER_URL=https://push.associahub.com.br
```

Você pode copiar o template:

```
cp android/gradle.properties.example android/gradle.properties
```

### 3. Background
- Service mantém conexão P2P
- Notificação mostra contagem de peers
- App pode ser fechado (service continua)

## 🐛 Debug

### Logcat

```bash
# Ver logs do app
adb logcat | grep ZapLivre

# Filtros específicos
adb logcat | grep "ZapLivreClient"
adb logcat | grep "ZapLivreService"
```

### Verificar biblioteca carregada

```bash
adb shell run-as com.zaplivre ls -l /data/data/com.zaplivre/lib/
```

### Verificar dados persistidos

```bash
adb shell run-as com.zaplivre ls -lR /data/data/com.zaplivre/files/zaplivre_data/
```

## ⚠️ Problemas Comuns

### `UnsatisfiedLinkError: couldn't find libzaplivre_core.so`

**Solução:**
1. Verificar se `.so` está em `jniLibs/arm64-v8a/`
2. Verificar se ABI do dispositivo é compatível (ARM64)
3. Fazer Clean Build: `Build > Clean Project` + `Build > Rebuild Project`

### `Failed to initialize ZapLivreClient`

**Possíveis causas:**
1. Permissões de storage negadas (Android 10+)
2. Keypair corrompida no storage
3. Biblioteca nativa incompatível

**Solução:**
```bash
# Limpar dados do app
adb shell pm clear com.zaplivre
```

### Service não inicia

**Verificar:**
1. Permissão POST_NOTIFICATIONS (Android 13+)
2. Bateria otimizada desabilitada para o app
3. Logs: `adb logcat | grep ZapLivreService`

## 🔒 Permissões

### Obrigatórias
- `INTERNET` - Comunicação P2P
- `ACCESS_NETWORK_STATE` - Detectar conectividade
- `FOREGROUND_SERVICE` - Service em background
- `POST_NOTIFICATIONS` - Notificações (Android 13+)

### Futuras (VoIP - FASE 12)
- `RECORD_AUDIO` - Chamadas de voz
- `CAMERA` - Videochamadas
- `BLUETOOTH_CONNECT` - Headsets Bluetooth

## 📦 Build Release

### 1. Gerar Keystore

```bash
keytool -genkey -v -keystore zaplivre-release.jks \
  -keyalg RSA -keysize 2048 -validity 10000 \
  -alias zaplivre
```

### 2. Configurar `keystore.properties`

```properties
storeFile=zaplivre-release.jks
storePassword=****
keyAlias=zaplivre
keyPassword=****
```

### 3. Build signed APK

```bash
./gradlew assembleRelease
```

APK gerada em: `app/build/outputs/apk/release/app-release.apk`

## 📊 Métricas

| Métrica | Valor |
|---------|-------|
| Min SDK | 26 (Android 8.0) |
| Target SDK | 34 (Android 14) |
| APK Size (debug) | ~10 MB |
| APK Size (release) | ~7 MB (com ProGuard) |
| Arquivos Kotlin | 11 |
| LoC Kotlin | ~1.500 |

## 🚀 Próximos Passos

- [ ] Implementar callbacks de eventos (message_received)
- [ ] Adicionar notificações de novas mensagens
- [ ] Implementar sistema de busca
- [ ] Adicionar suporte para grupos
- [ ] Adicionar envio de imagens
- [ ] Implementar VoIP (FASE 12)

## 📚 Recursos

- [Jetpack Compose](https://developer.android.com/jetpack/compose)
- [Material3 Design](https://m3.material.io/)
- [UniFFI](https://mozilla.github.io/uniffi-rs/)
- [Kotlin Coroutines](https://kotlinlang.org/docs/coroutines-overview.html)

---

**Versão:** 0.1.0-alpha
**Última atualização:** 2025-01-20
