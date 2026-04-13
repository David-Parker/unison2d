use tempfile::tempdir;
use unison_cli::commands::test::run_with;
use unison_cli::config::{Build, Config, Engine, Lang, Platforms, Project};
use unison_cli::toolchain::MockInvoker;

fn cfg(lang: Lang) -> Config {
    Config {
        project: Project { name: "g".into(), lang },
        engine: Engine { git: "x".into(), tag: Some("v0".into()), branch: None, rev: None, link_path: None },
        platforms: Platforms { web: true, ios: false, android: false },
        build: Build::default(),
    }
}

#[test]
fn test_lua_runs_cargo_only() {
    let dir = tempdir().unwrap();
    let mock = MockInvoker::new();
    run_with(&cfg(Lang::Lua), dir.path(), &mock).unwrap();
    mock.assert_called("cargo", &["test"]);
    assert_eq!(mock.invocations().len(), 1);
}

#[test]
fn test_ts_runs_cargo_and_npm() {
    let dir = tempdir().unwrap();
    let mock = MockInvoker::new();
    run_with(&cfg(Lang::Ts), dir.path(), &mock).unwrap();
    mock.assert_called("cargo", &["test"]);
    mock.assert_called("npm", &["test"]);
}
