# Guia de Build - ZapLivre Android

Documentação step-by-step do processo de build do app Android ZapLivre.

## 📦 Processo de Build Completo

### Etapa 1: Preparar Artefatos do Core

Antes de compilar o Android app, é necessário ter os bindings e bibliotecas nativas.

```bash
# 1. Navegar para o diretório core
cd /Users/edsonmartins/desenvolvimento/zaplivre/core

# 2. Gerar bindings Kotlin e Swift
cargo run --example generate_bindings

# Saída esperada:
# ✓ Bindings generated successfully!
# Output directory: target/bindings
# - Kotlin: target/bindings/uniffi/zaplivre/zaplivre.kt
# - Swift: target/bindings/zaplivre.swift

# 3. Compilar biblioteca nativa para Android ARM64
export CC_aarch64_linux_android="$HOME/Library/Android/sdk/ndk/26.3.11579264/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android33-clang"
export AR_aarch64_linux_android="$HOME/Library/Android/sdk/ndk/26.3.11579264/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ar"

cargo build --target aarch64-linux-android --release --lib

# Saída esperada:
# Finished `release` profile [optimized] target(s) in 2m 47s

# 4. Verificar artefatos gerados
ls -lh target/bindings/uniffi/zaplivre/zaplivre.kt
# -rw-r--r--  1 user  staff    80K ... zaplivre.kt

ls -lh target/aarch64-linux-android/release/libzaplivre_core.so
# -rwxr-xr-x@ 1 user  staff   6.3M ... libzaplivre_core.so
```

**✅ Checkpoint 1:** Artefatos do core prontos

### Etapa 2: Copiar Artefatos para Projeto Android

```bash
# 1. Criar diretórios necessários (se não existirem)
mkdir -p ../android/app/src/main/kotlin/uniffi/zaplivre
mkdir -p ../android/app/src/main/jniLibs/arm64-v8a

# 2. Copiar bindings Kotlin
cp target/bindings/uniffi/zaplivre/zaplivre.kt \
   ../android/app/src/main/kotlin/uniffi/zaplivre/

# 3. Copiar biblioteca nativa
cp target/aarch64-linux-android/release/libzaplivre_core.so \
   ../android/app/src/main/jniLibs/arm64-v8a/

# 4. Verificar cópias
ls -lh ../android/app/src/main/kotlin/uniffi/zaplivre/zaplivre.kt
ls -lh ../android/app/src/main/jniLibs/arm64-v8a/libzaplivre_core.so
```

**✅ Checkpoint 2:** Artefatos copiados para Android

### Etapa 3: Build do Android App

#### Método 1: Android Studio (Recomendado)

```bash
# 1. Abrir projeto
cd ../android
open -a "Android Studio" .

# 2. Aguardar Gradle Sync
# - Automático ao abrir projeto
# - Barra de progresso no topo
# - Aguardar "Gradle sync finished" (1-3 min)

# 3. Build via menu
# Build > Make Project (Cmd+F9)

# Ou via terminal integrado:
./gradlew assembleDebug
```

**Saída esperada (Gradle):**
```
> Configure project :app
> Task :app:preBuild
> Task :app:preDebugBuild
> Task :app:compileDebugKotlin
> Task :app:mergeDebugJniLibFolders        ← IMPORTANTE
> Task :app:compileDebugJavaWithJavac
> Task :app:mergeDebugAssets
> Task :app:processDebugManifest
> Task :app:packageDebug
> Task :app:assembleDebug

BUILD SUCCESSFUL in 1m 23s
89 actionable tasks: 89 executed
```

#### Método 2: Terminal (linha de comando)

```bash
cd android

# Build debug (desenvolvimento)
./gradlew assembleDebug

# Build release (produção - requer keystore)
./gradlew assembleRelease

# Limpar build anterior
./gradlew clean

# Build completo (limpar + build)
./gradlew clean assembleDebug
```

**✅ Checkpoint 3:** Build successful

### Etapa 4: Localizar APK Gerada

```bash
# APK debug
ls -lh app/build/outputs/apk/debug/app-debug.apk
# -rw-r--r--  1 user  staff    10M ... app-debug.apk

# APK release (se compilou)
ls -lh app/build/outputs/apk/release/app-release-unsigned.apk
```

### Etapa 5: Instalar no Device/Emulador

#### Opção A: Via Android Studio

```
1. Conectar device ou iniciar emulador
2. Clicar em Run (▶️) ou Ctrl+R / Cmd+R
3. Aguardar instalação
```

#### Opção B: Via ADB (linha de comando)

```bash
# 1. Verificar devices conectados
adb devices
# List of devices attached
# emulator-5554    device

# 2. Instalar APK
adb install app/build/outputs/apk/debug/app-debug.apk

# Ou usar Gradle (mais fácil)
./gradlew installDebug

# 3. Verificar instalação
adb shell pm list packages | grep zaplivre
# package:com.zaplivre

# 4. Executar app
adb shell am start -n com.zaplivre/.MainActivity
```

**✅ Checkpoint 4:** App instalado

## 🔍 Verificação Pós-Build

### Verificar Conteúdo do APK

```bash
# Extrair APK para inspecionar
unzip -l app/build/outputs/apk/debug/app-debug.apk | grep -E "(libzaplivre|uniffi)"

# Esperado:
# lib/arm64-v8a/libzaplivre_core.so        ← Biblioteca nativa
# classes.dex                              ← Código compilado (inclui UniFFI)
```

### Verificar Tamanho do APK

```bash
du -h app/build/outputs/apk/debug/app-debug.apk
# ~10M (debug)
# ~7M  (release com ProGuard)
```

**Breakdown do tamanho:**
- libzaplivre_core.so: ~6.3 MB
- Kotlin bindings: ~80 KB compilado
- Jetpack Compose: ~2 MB
- JNA (UniFFI): ~1 MB
- Recursos/assets: ~500 KB

### Verificar Símbolos Nativos

```bash
# Extrair .so do APK
unzip -j app/build/outputs/apk/debug/app-debug.apk \
  lib/arm64-v8a/libzaplivre_core.so -d /tmp/

# Verificar símbolos UniFFI
nm -D /tmp/libzaplivre_core.so | grep uniffi

# Esperado:
# uniffi_zaplivre_core_fn_init_callback_vtable_zaplivreclient
# uniffi_zaplivre_core_fn_constructor_zaplivreclient_new
# ...
```

## 🏗️ Build Variants

### Debug Build

**Características:**
- Código não otimizado
- Símbolos de debug incluídos
- ProGuard desabilitado
- Tamanho maior (~10 MB)
- Mais fácil de debugar

**Comando:**
```bash
./gradlew assembleDebug
```

**Quando usar:**
- Desenvolvimento
- Testes
- Debug de crashes

### Release Build

**Características:**
- Código otimizado
- ProGuard habilitado
- Símbolos stripped
- Tamanho menor (~7 MB)
- Requer signing (keystore)

**Comando:**
```bash
./gradlew assembleRelease
```

**Quando usar:**
- Produção
- Google Play Store
- Testes de performance

## 🔧 Build Configuration

### build.gradle.kts (app level)

```kotlin
android {
    compileSdk = 34

    defaultConfig {
        applicationId = "com.zaplivre"
        minSdk = 26
        targetSdk = 34
        versionCode = 1
        versionName = "0.1.0-alpha"

        // CRÍTICO: Especificar ABI
        ndk {
            abiFilters += listOf("arm64-v8a")
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true  // ProGuard
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }
}
```

### gradle.properties

```properties
# Performance
org.gradle.jvmargs=-Xmx2048m
org.gradle.caching=true
org.gradle.configuration-cache=true

# Android
android.useAndroidX=true
android.nonTransitiveRClass=true
```

## 🐛 Troubleshooting de Build

### Erro 1: "Unsupported class file major version 65"

**Causa:** Gradle usando JDK incompatível

**Solução:**
```bash
# Verificar JDK
java -version
# Deve ser JDK 17

# Android Studio: File > Project Structure
# Gradle Settings > Gradle JDK: selecionar JDK 17
```

### Erro 2: "Could not find libzaplivre_core.so"

**Causa:** Biblioteca não copiada ou ABI incorreta

**Solução:**
```bash
# Verificar se existe
ls app/src/main/jniLibs/arm64-v8a/libzaplivre_core.so

# Se não existir, copiar novamente
cp ../core/target/aarch64-linux-android/release/libzaplivre_core.so \
   app/src/main/jniLibs/arm64-v8a/

# Rebuild
./gradlew clean assembleDebug
```

### Erro 3: "Duplicate class uniffi.zaplivre.*"

**Causa:** Bindings duplicados ou em local errado

**Solução:**
```bash
# Verificar se está no local correto
ls app/src/main/kotlin/uniffi/zaplivre/zaplivre.kt

# Remover duplicatas
find app/src -name "zaplivre.kt" | grep -v "uniffi/zaplivre"
# Se encontrar outros, deletar
```

### Erro 4: "Task :app:mergeDebugJniLibFolders FAILED"

**Causa:** Problema ao copiar JNI libs

**Solução:**
```bash
# Limpar diretórios de build
rm -rf app/build
rm -rf .gradle

# Rebuild
./gradlew clean
./gradlew assembleDebug
```

### Erro 5: Gradle Sync Failed

**Solução:**
```bash
# 1. Limpar cache
rm -rf ~/.gradle/caches/

# 2. Redownload dependencies
./gradlew --refresh-dependencies

# 3. Invalidar cache do Android Studio
# File > Invalidate Caches > Invalidate and Restart
```

## 📊 Build Performance

### Tempos Esperados

| Operação | Primeira Vez | Subsequente |
|----------|--------------|-------------|
| Gradle Sync | 2-5 min | 10-30 seg |
| Clean Build | 3-5 min | - |
| Incremental Build | - | 30-60 seg |
| Hot Reload (Compose) | - | 1-3 seg |

### Otimizações

**gradle.properties:**
```properties
# Aumentar heap do Gradle
org.gradle.jvmargs=-Xmx4096m

# Parallel build
org.gradle.parallel=true

# Daemon
org.gradle.daemon=true

# Configuration cache
org.gradle.configuration-cache=true
```

**~/.gradle/gradle.properties (global):**
```properties
org.gradle.jvmargs=-Xmx4096m -XX:MaxMetaspaceSize=1024m
org.gradle.parallel=true
org.gradle.caching=true
```

## 🚀 Build Script Automatizado

Criar script `build.sh` na raiz do projeto Android:

```bash
#!/bin/bash
# Build script para ZapLivre Android

set -e  # Exit on error

echo "🔨 ZapLivre Android Build Script"
echo "================================"

# 1. Verificar pré-requisitos
echo "📋 Verificando pré-requisitos..."
if [ ! -f "app/src/main/jniLibs/arm64-v8a/libzaplivre_core.so" ]; then
    echo "❌ libzaplivre_core.so não encontrada!"
    echo "Execute primeiro:"
    echo "  cd ../core && cargo run --example generate_bindings"
    exit 1
fi

if [ ! -f "app/src/main/kotlin/uniffi/zaplivre/zaplivre.kt" ]; then
    echo "❌ Bindings Kotlin não encontrados!"
    exit 1
fi

echo "✅ Pré-requisitos OK"

# 2. Limpar build anterior
echo ""
echo "🧹 Limpando build anterior..."
./gradlew clean

# 3. Build
echo ""
echo "🔨 Compilando..."
./gradlew assembleDebug

# 4. Verificar resultado
APK_PATH="app/build/outputs/apk/debug/app-debug.apk"
if [ -f "$APK_PATH" ]; then
    echo ""
    echo "✅ Build concluído com sucesso!"
    echo "📦 APK gerado: $APK_PATH"
    echo "📏 Tamanho: $(du -h $APK_PATH | cut -f1)"

    # Opcional: Instalar automaticamente se device conectado
    if adb devices | grep -q "device$"; then
        echo ""
        read -p "Instalar no device conectado? (y/n) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            ./gradlew installDebug
            echo "✅ App instalado!"
        fi
    fi
else
    echo "❌ Build falhou!"
    exit 1
fi
```

**Usar:**
```bash
chmod +x build.sh
./build.sh
```

## 📝 Build Checklist

Antes de fazer build:

- [ ] JDK 17 instalado e configurado
- [ ] Android SDK API 34 instalado
- [ ] NDK 26.3.11579264 instalado
- [ ] Gradle wrapper presente (gradle/wrapper/)
- [ ] libzaplivre_core.so em jniLibs/arm64-v8a/
- [ ] zaplivre.kt em kotlin/uniffi/zaplivre/
- [ ] gradle.properties configurado
- [ ] build.gradle.kts sem erros de sintaxe

Durante build:

- [ ] Gradle sync successful
- [ ] Nenhum erro de compilação
- [ ] Task mergeDebugJniLibFolders OK
- [ ] APK gerado em outputs/apk/

Após build:

- [ ] APK existe e tem ~10 MB
- [ ] libzaplivre_core.so está dentro do APK
- [ ] App instala sem erros
- [ ] App abre sem crash
- [ ] Biblioteca nativa carrega (check logs)

---

**Última atualização:** 2025-01-20
**Testado em:** macOS Sonoma 14.4, Android Studio Hedgehog
