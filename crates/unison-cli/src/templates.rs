use include_dir::{include_dir, Dir};

pub static COMMON: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates/common");
pub static SCRIPTING_LUA: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates/scripting-lua");
pub static SCRIPTING_TS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates/scripting-ts");
pub static PLATFORM_WEB: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates/platform-web");
pub static PLATFORM_IOS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates/platform-ios");
pub static PLATFORM_ANDROID: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates/platform-android");

/// Engine-side TypeScript type declarations. Copied into each scaffolded TS
/// project under `project/scripts-src/types/unison2d/` so the TS compiler can
/// resolve globals like `engine`, `input`, `World`, etc. Pinned to whatever
/// engine version the CLI was built against.
pub static ENGINE_TYPES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../unison-scripting/types");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_contains_cargo_toml() {
        assert!(COMMON.get_file("Cargo.toml").is_some());
        assert!(COMMON.get_file("build.rs").is_some());
        assert!(COMMON.get_file("project/lib.rs").is_some());
        assert!(COMMON.get_file(".gitignore").is_some());
    }

    #[test]
    fn scripting_lua_contains_main() {
        assert!(SCRIPTING_LUA.get_file("project/assets/scripts/main.lua").is_some());
    }

    #[test]
    fn scripting_ts_contains_core_files() {
        assert!(SCRIPTING_TS.get_file("package.json").is_some());
        assert!(SCRIPTING_TS.get_file("project/scripts-src/tsconfig.json").is_some());
        assert!(SCRIPTING_TS.get_file("project/scripts-src/main.ts").is_some());
        assert!(SCRIPTING_TS.get_file(".gitignore-ts-addon").is_some());
    }

    #[test]
    fn platform_web_contains_index_and_trunk() {
        assert!(PLATFORM_WEB.get_file("index.html").is_some());
        assert!(PLATFORM_WEB.get_file("Trunk.toml").is_some());
    }

    #[test]
    fn platform_ios_contains_core_files() {
        assert!(PLATFORM_IOS.get_file("AppDelegate.swift").is_some());
        assert!(PLATFORM_IOS.get_file("Info.plist").is_some());
        assert!(PLATFORM_IOS.get_file("Base.lproj/Main.storyboard").is_some());
    }

    #[test]
    fn platform_android_contains_core_files() {
        assert!(PLATFORM_ANDROID.get_file("settings.gradle.kts").is_some());
        assert!(PLATFORM_ANDROID.get_file("app/src/main/AndroidManifest.xml").is_some());
        assert!(PLATFORM_ANDROID.get_file("build-rust.sh").is_some());
    }

    #[test]
    fn engine_types_contains_globals() {
        assert!(ENGINE_TYPES.get_file("game.d.ts").is_some());
        assert!(ENGINE_TYPES.get_file("engine.d.ts").is_some());
        assert!(ENGINE_TYPES.get_file("input.d.ts").is_some());
        assert!(ENGINE_TYPES.get_file("index.d.ts").is_some());
    }
}
