use tempfile::tempdir;
use unison_cli::commands::build::{run_with, BuildArgs};
use unison_cli::config::{Build, Config, Engine, Lang, Platforms, Project};
use unison_cli::toolchain::MockInvoker;

fn ios_cfg(name: &str) -> Config {
    Config {
        project: Project { name: name.to_string(), lang: Lang::Lua },
        engine: Engine { git: "x".into(), tag: Some("v0".into()), branch: None, rev: None, link_path: None },
        platforms: Platforms { web: false, ios: true, android: false },
        build: Build::default(),
    }
}

#[test]
fn build_ios_invokes_xcodebuild_debug() {
    let dir = tempdir().unwrap();
    let mock = MockInvoker::new();
    run_with(&ios_cfg("g"), dir.path(), &mock, BuildArgs {
        platform: "ios".into(), release: false, profile: false,
    }).unwrap();
    mock.assert_called("xcodebuild", &[
        "-project", "platform/ios/g-ios.xcodeproj",
        "-scheme", "g-ios",
        "-configuration", "Debug",
        "build",
    ]);
}

#[test]
fn build_ios_release_passes_release_config() {
    let dir = tempdir().unwrap();
    let mock = MockInvoker::new();
    run_with(&ios_cfg("g"), dir.path(), &mock, BuildArgs {
        platform: "ios".into(), release: true, profile: false,
    }).unwrap();
    let call = &mock.invocations()[0];
    assert!(call.args.iter().any(|a| a == "Release"));
}
