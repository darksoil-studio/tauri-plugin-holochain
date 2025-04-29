# Project Setup for Android Development

> [!NOTE]
> This guide assumes that you have already gone through either [how to create an end-user hApp](../how-to-create-an-end-user-happ.md) or [how to create a holochain runtime](../how-to-create-a-holochain-runtime.md).

1. In the root folder of your repository, run:

::: code-group
```bash [npm]
npm run tauri android init -- --skip-targets-install
```

```bash [yarn]
yarn tauri android init -- --skip-targets-install
```

```bash [pnpm]
pnpm tauri android init -- --skip-targets-install
```
:::

This should initialize all the necessary android files for your app.

2. Go in the `src-tauri/gen/android/app/build.gradle.kts` that was generated in the previous step, and set the "usesCleartextTraffic" to true:

```kotlin
plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("rust")
}

android {
    compileSdk = 34
    namespace = "com.tauri.tauri_app"
    defaultConfig {
        manifestPlaceholders["usesCleartextTraffic"] = "false" // [!code --]
        manifestPlaceholders["usesCleartextTraffic"] = "true" // [!code ++]
        applicationId = "com.tauri.tauri_app"
        minSdk = 24
        targetSdk = 34
        versionCode = 1
        versionName = "1.0"
    }

    buildTypes {
        getByName("debug") {
            manifestPlaceholders["usesCleartextTraffic"] = "true"
            isDebuggable = true
            isJniDebuggable = true
            isMinifyEnabled = false
            packaging {
                jniLibs.keepDebugSymbols.add("*/arm64-v8a/*.so")
                jniLibs.keepDebugSymbols.add("*/armeabi-v7a/*.so")
                jniLibs.keepDebugSymbols.add("*/x86/*.so")
                jniLibs.keepDebugSymbols.add("*/x86_64/*.so")
            }
        }
        getByName("release") {
            signingConfig = signingConfigs.getByName("release")
            isMinifyEnabled = true
            proguardFiles(
                *fileTree(".") { include("**/*.pro") }
                    .plus(getDefaultProguardFile("proguard-android-optimize.txt"))
                    .toList().toTypedArray()
            )
        }
    }
    kotlinOptions {
        jvmTarget = "1.8"
    }
}

rust {
    rootDirRel = "../../../"
}

dependencies {
    implementation("androidx.webkit:webkit:1.6.1")
    implementation("androidx.appcompat:appcompat:1.6.1")
    implementation("com.google.android.material:material:1.8.0")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.4")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.0")
}

apply(from = "tauri.build.gradle.kts")
```

--- 

That's it! Your project is now ready to develop for the Android platform.

Continue to [Device Setup](./device-setup) to learn how to setup an Android device for testing your hApp.
