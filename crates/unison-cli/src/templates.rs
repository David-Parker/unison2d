use include_dir::{include_dir, Dir};

pub static COMMON: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates/common");
pub static SCRIPTING_LUA: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates/scripting-lua");
pub static PLATFORM_WEB: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates/platform-web");

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
    fn platform_web_contains_index_and_trunk() {
        assert!(PLATFORM_WEB.get_file("index.html").is_some());
        assert!(PLATFORM_WEB.get_file("Trunk.toml").is_some());
    }
}
