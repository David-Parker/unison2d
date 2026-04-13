use tempfile::tempdir;
use unison_cli::commands::dev::run_with;
use unison_cli::config::{Build, Config, Engine, Lang, Platforms, Project};
use unison_cli::toolchain::MockInvoker;

fn lua_web() -> Config {
    Config {
        project: Project { name: "g".into(), lang: Lang::Lua },
        engine: Engine { git: "x".into(), tag: Some("v0".into()), branch: None, rev: None, link_path: None },
        platforms: Platforms { web: true, ios: false, android: false },
        build: Build::default(),
    }
}

#[test]
fn dev_web_invokes_trunk_serve() {
    let dir = tempdir().unwrap();
    let mock = MockInvoker::new();
    run_with(&lua_web(), dir.path(), &mock, "web").unwrap();
    mock.assert_called("trunk", &["serve"]);
}

#[test]
fn dev_web_ts_also_runs_tstl_watch() {
    let mut cfg = lua_web();
    cfg.project.lang = Lang::Ts;
    let dir = tempdir().unwrap();
    let mock = MockInvoker::new();
    run_with(&cfg, dir.path(), &mock, "web").unwrap();
    mock.assert_called("npx", &["tstl", "-p", "project/scripts-src/tsconfig.json", "--watch"]);
    mock.assert_called("trunk", &["serve"]);
}

#[test]
fn dev_ios_prints_hint_and_succeeds() {
    let mut cfg = lua_web();
    cfg.platforms.ios = true;
    let dir = tempdir().unwrap();
    let mock = MockInvoker::new();
    // No Invoker calls expected — just a println. Should return Ok(()).
    run_with(&cfg, dir.path(), &mock, "ios").unwrap();
    assert_eq!(mock.invocations().len(), 0);
}

#[test]
fn dev_android_prints_hint_and_succeeds() {
    let mut cfg = lua_web();
    cfg.platforms.android = true;
    let dir = tempdir().unwrap();
    let mock = MockInvoker::new();
    run_with(&cfg, dir.path(), &mock, "android").unwrap();
    assert_eq!(mock.invocations().len(), 0);
}
