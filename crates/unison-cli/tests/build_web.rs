use tempfile::tempdir;
use unison_cli::commands::build::{run_with, BuildArgs};
use unison_cli::config::{Build, Config, Engine, Lang, Platforms, Project};
use unison_cli::toolchain::MockInvoker;

fn lua_web_config(name: &str) -> Config {
    Config {
        project: Project { name: name.to_string(), lang: Lang::Lua },
        engine: Engine {
            git: "https://x".into(), tag: Some("v0".into()),
            branch: None, rev: None, link_path: None,
        },
        platforms: Platforms { web: true, ios: false, android: false },
        build: Build::default(),
    }
}

#[test]
fn build_web_invokes_trunk() {
    let dir = tempdir().unwrap();
    let cfg = lua_web_config("g");
    let mock = MockInvoker::new();
    run_with(&cfg, dir.path(), &mock, BuildArgs {
        platform: "web".into(), release: false, profile: false,
    }).unwrap();
    mock.assert_called("trunk", &["build"]);
}

#[test]
fn build_web_release_passes_flag() {
    let dir = tempdir().unwrap();
    let cfg = lua_web_config("g");
    let mock = MockInvoker::new();
    run_with(&cfg, dir.path(), &mock, BuildArgs {
        platform: "web".into(), release: true, profile: false,
    }).unwrap();
    mock.assert_called("trunk", &["build", "--release"]);
}

#[test]
fn build_web_ts_runs_tstl_first() {
    let dir = tempdir().unwrap();
    let mut cfg = lua_web_config("g");
    cfg.project.lang = Lang::Ts;
    let mock = MockInvoker::new();
    run_with(&cfg, dir.path(), &mock, BuildArgs {
        platform: "web".into(), release: false, profile: false,
    }).unwrap();
    let calls = mock.invocations();
    assert_eq!(calls[0].program, "npx");
    assert_eq!(calls[0].args[0], "tstl");
    assert_eq!(calls[1].program, "trunk");
}
