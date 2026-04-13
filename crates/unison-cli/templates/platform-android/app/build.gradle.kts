plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "{{ANDROID_APP_ID}}"
    compileSdk = 34

    defaultConfig {
        applicationId = "{{ANDROID_APP_ID}}"
        minSdk = 24
        targetSdk = 34
        versionCode = 1
        versionName = "0.1.0"

        ndk {
            abiFilters += listOf("arm64-v8a", "x86_64")
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            signingConfig = signingConfigs.getByName("debug")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }

    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }

    packaging {
        jniLibs {
            useLegacyPackaging = true
        }
    }
}

dependencies {
    implementation(project(":unison-android"))
}

// Auto-compile Rust native library before packaging. Mirrors the Xcode
// "Run Script" build phase that calls cargo build.
val buildRust by tasks.registering(Exec::class) {
    workingDir = rootProject.projectDir
    val profile = if (gradle.startParameter.taskNames.any { it.contains("Release", ignoreCase = true) }) "release" else "debug"
    commandLine("bash", "./build-rust.sh", profile)
}

// Run build-rust.sh before the .so files get merged into the APK.
tasks.matching { it.name.startsWith("merge") && it.name.contains("JniLibFolders") }.configureEach {
    dependsOn(buildRust)
}
