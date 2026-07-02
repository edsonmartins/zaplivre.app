plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    // id("com.google.gms.google-services") // Disabled for now - P2P app doesn't need Firebase
}

android {
    namespace = "com.mepassa"
    compileSdk = 34

    defaultConfig {
        applicationId = "com.mepassa"
        minSdk = 26  // Android 8.0 (necessário para suporte bom de foreground services)
        targetSdk = 34
        versionCode = 1
        versionName = "0.1.0-alpha"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        vectorDrawables {
            useSupportLibrary = true
        }

        // Garantir que JNI libs sejam incluídas
        // arm64-v8a: devices reais | x86_64: emulador
        ndk {
            abiFilters += listOf("arm64-v8a", "x86_64")
        }

        val messageStoreUrl = (project.findProperty("MESSAGE_STORE_URL") as String?)
            ?: System.getenv("MESSAGE_STORE_URL")
            ?: "https://store.associahub.com.br"
        buildConfigField("String", "MESSAGE_STORE_URL", "\"$messageStoreUrl\"")

        val pushServerUrl = (project.findProperty("PUSH_SERVER_URL") as String?)
            ?: System.getenv("PUSH_SERVER_URL")
            ?: "https://push.associahub.com.br"
        buildConfigField("String", "PUSH_SERVER_URL", "\"$pushServerUrl\"")

        val signalingServerUrl = (project.findProperty("SIGNALING_SERVER_URL") as String?)
            ?: System.getenv("SIGNALING_SERVER_URL")
            ?: "wss://signaling.associahub.com.br/ws"
        buildConfigField("String", "SIGNALING_SERVER_URL", "\"$signalingServerUrl\"")
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
        debug {
            isDebuggable = true
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
        freeCompilerArgs += listOf(
            "-opt-in=androidx.compose.foundation.ExperimentalFoundationApi",
            "-opt-in=androidx.compose.material3.ExperimentalMaterial3Api",
            "-opt-in=kotlin.RequiresOptIn"
        )
    }

    buildFeatures {
        compose = true
        buildConfig = true
    }

    composeOptions {
        kotlinCompilerExtensionVersion = "1.5.7"
    }

    packaging {
        resources {
            excludes += "/META-INF/{AL2.0,LGPL2.1}"
        }
    }

    sourceSets {
        getByName("main") {
            // Include Kotlin source directory
            java.srcDirs("src/main/kotlin")
        }
    }
}

// Build da lib nativa Rust (libmepassa_core.so) antes do build Android.
// Por padrão roda apenas se as .so ainda não existem; force com -PrebuildNative.
val jniLibsDir = file("src/main/jniLibs")
val buildRustCore = tasks.register<Exec>("buildRustCore") {
    group = "build"
    description = "Compila libmepassa_core.so via cargo (android/build-native.sh)"
    workingDir = rootDir.parentFile
    commandLine("bash", "android/build-native.sh")
    onlyIf {
        project.hasProperty("rebuildNative") ||
            !jniLibsDir.resolve("arm64-v8a/libmepassa_core.so").exists() ||
            !jniLibsDir.resolve("x86_64/libmepassa_core.so").exists()
    }
}

tasks.named("preBuild") {
    dependsOn(buildRustCore)
}

dependencies {
    // AndroidX Core
    implementation("androidx.core:core-ktx:1.12.0")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.7.0")
    implementation("androidx.activity:activity-compose:1.8.2")

    // Jetpack Compose
    val composeBom = platform("androidx.compose:compose-bom:2023.10.01")
    implementation(composeBom)
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-graphics")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-extended")

    // Navigation Compose
    implementation("androidx.navigation:navigation-compose:2.7.6")

    // ViewModel Compose
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.7.0")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.7.0")

    // Coroutines
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.7.3")

    // DataStore (para salvar configs localmente)
    implementation("androidx.datastore:datastore-preferences:1.0.0")

    // Security Crypto (EncryptedSharedPreferences)
    implementation("androidx.security:security-crypto:1.1.0-alpha06")

    // Accompanist (permissions, etc)
    implementation("com.google.accompanist:accompanist-permissions:0.32.0")

    // CameraX (FASE 14 - Video Calls)
    val cameraxVersion = "1.3.1"
    implementation("androidx.camera:camera-core:$cameraxVersion")
    implementation("androidx.camera:camera-camera2:$cameraxVersion")
    implementation("androidx.camera:camera-lifecycle:$cameraxVersion")
    implementation("androidx.camera:camera-view:$cameraxVersion")

    // Photo Picker & Image Loading (FASE 16 - Media)
    implementation("androidx.activity:activity-ktx:1.8.2")
    implementation("io.coil-kt:coil-compose:2.5.0")
    implementation("io.coil-kt:coil-gif:2.5.0")

    // Firebase (Push Notifications)
    implementation(platform("com.google.firebase:firebase-bom:32.7.0"))
    implementation("com.google.firebase:firebase-messaging-ktx")

    // HTTP Client (para Push Server)
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
    implementation("com.squareup.okhttp3:logging-interceptor:4.12.0")

    // JNA (necessário para UniFFI)
    implementation("net.java.dev.jna:jna:5.14.0@aar")

    // Testing
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.5")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.1")
    androidTestImplementation(composeBom)
    androidTestImplementation("androidx.compose.ui:ui-test-junit4")

    // Debug
    debugImplementation("androidx.compose.ui:ui-tooling")
    debugImplementation("androidx.compose.ui:ui-test-manifest")
}
