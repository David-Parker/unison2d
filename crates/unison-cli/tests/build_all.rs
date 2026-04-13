use tempfile::tempdir;
use unison_cli::commands::build::{run_with, BuildArgs};
use unison_cli::config::{Build, Config, Engine, Lang, Platforms, Project};
use unison_cli::toolchain::MockInvoker;

#[test]
fn build_all_runs_every_enabled_platform() {
    let cfg = Config {
        project: Project { name: "g".into(), lang: Lang::Lua },
        engine: Engine { git: "x".into(), tag: Some("v0".into()), branch: None, rev: None, link_path: None },
        platforms: Platforms { web: true, ios: true, android: true },
        build: Build::default(),
    };
    let dir = tempdir().unwrap();
    let mock = MockInvoker::new();
    run_with(&cfg, dir.path(), &mock, BuildArgs {
        platform: "all".into(), release: false, profile: false,
    }).unwrap();
    let calls = mock.invocations();
    assert!(calls.iter().any(|c| c.program == "trunk"));
    assert!(calls.iter().any(|c| c.program == "xcodebuild"));
    assert!(calls.iter().any(|c| c.program == "./gradlew"));
}

#[test]
fn build_all_ts_runs_tstl_once() {
    let cfg = Config {
        project: Project { name: "g".into(), lang: Lang::Ts },
        engine: Engine { git: "x".into(), tag: Some("v0".into()), branch: None, rev: None, link_path: None },
        platforms: Platforms { web: true, ios: true, android: true },
        build: Build::default(),
    };
    let dir = tempdir().unwrap();
    let mock = MockInvoker::new();
    run_with(&cfg, dir.path(), &mock, BuildArgs {
        platform: "all".into(), release: false, profile: false,
    }).unwrap();
    let calls = mock.invocations();
    let npx_calls: Vec<_> = calls.iter().filter(|c| c.program == "npx").collect();
    assert_eq!(npx_calls.len(), 1, "tstl should be invoked exactly once, got: {:?}", calls);
}

#[test]
fn build_all_skips_disabled_platforms() {
    let cfg = Config {
        project: Project { name: "g".into(), lang: Lang::Lua },
        engine: Engine { git: "x".into(), tag: Some("v0".into()), branch: None, rev: None, link_path: None },
        platforms: Platforms { web: true, ios: false, android: false },
        build: Build::default(),
    };
    let dir = tempdir().unwrap();
    let mock = MockInvoker::new();
    run_with(&cfg, dir.path(), &mock, BuildArgs {
        platform: "all".into(), release: false, profile: false,
    }).unwrap();
    let calls = mock.invocations();
    assert!(calls.iter().any(|c| c.program == "trunk"));
    assert!(!calls.iter().any(|c| c.program == "xcodebuild"));
    assert!(!calls.iter().any(|c| c.program == "./gradlew"));
}
