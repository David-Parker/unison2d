use tempfile::tempdir;
use unison_cli::commands::build::{run_with, BuildArgs};
use unison_cli::config::{Build, Config, Engine, Lang, Platforms, Project};
use unison_cli::toolchain::MockInvoker;

fn android_cfg() -> Config {
    Config {
        project: Project { name: "g".into(), lang: Lang::Lua },
        engine: Engine { git: "x".into(), tag: Some("v0".into()), branch: None, rev: None, link_path: None },
        platforms: Platforms { web: false, ios: false, android: true },
        build: Build::default(),
    }
}

#[test]
fn build_android_debug() {
    let dir = tempdir().unwrap();
    let mock = MockInvoker::new();
    run_with(&android_cfg(), dir.path(), &mock, BuildArgs {
        platform: "android".into(), release: false, profile: false,
    }).unwrap();
    mock.assert_called("./gradlew", &[":app:assembleDebug"]);
}

#[test]
fn build_android_release() {
    let dir = tempdir().unwrap();
    let mock = MockInvoker::new();
    run_with(&android_cfg(), dir.path(), &mock, BuildArgs {
        platform: "android".into(), release: true, profile: false,
    }).unwrap();
    mock.assert_called("./gradlew", &[":app:assembleRelease"]);
}

#[test]
fn build_android_profile_adds_gradle_prop() {
    let dir = tempdir().unwrap();
    let mock = MockInvoker::new();
    run_with(&android_cfg(), dir.path(), &mock, BuildArgs {
        platform: "android".into(), release: false, profile: true,
    }).unwrap();
    let call = &mock.invocations()[0];
    assert!(call.args.iter().any(|a| a == "-Pprofile=true"));
}
