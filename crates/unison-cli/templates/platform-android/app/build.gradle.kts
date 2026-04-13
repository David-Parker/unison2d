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
        versionName = "1.0"
    }
    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
}

// Cross-compile Rust before packaging APK.
val buildRust = tasks.register<Exec>("buildRust") {
    workingDir = rootProject.projectDir
    commandLine = listOf("./build-rust.sh")
}
tasks.matching { it.name.startsWith("merge") && it.name.endsWith("JniLibFolders") }
    .configureEach { dependsOn(buildRust) }

dependencies {
    implementation("androidx.core:core-ktx:1.12.0")
    implementation("androidx.appcompat:appcompat:1.6.1")
    // UnisonAndroid — JVM package pulled from engine git. Gradle setup for git deps TBD by Task 17 wiring.
}
