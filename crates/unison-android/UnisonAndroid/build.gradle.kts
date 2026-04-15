plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "com.unison2d"
    compileSdk = 34

    defaultConfig {
        // API 26+ required: cpal (via unison-audio → kira) links libaaudio,
        // which only exists in the NDK sysroot from API 26 onwards.
        // Also required for AudioFocusRequest (consumers wiring AudioFocus).
        minSdk = 26
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }
}
