pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "{{PROJECT_NAME}}-android"

include(":app")
include(":unison-android")
// Path to the engine's UnisonAndroid module. Rewritten by `unison link`
// to the absolute path of the linked engine workspace; restored here by
// `unison unlink`. Unlinked templates assume `unison2d/` sits next to
// the project (not the common case — most users will need to link first).
project(":unison-android").projectDir =
    file("../../unison2d/crates/unison-android/UnisonAndroid")
