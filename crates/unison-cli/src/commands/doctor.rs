use anyhow::Result;
use std::path::Path;
use std::process::Command;

use crate::config::{Config, Lang};

#[derive(Debug)]
struct Check {
    name: &'static str,
    present: bool,
    install_hint: &'static str,
}

pub fn run(project_root: Option<&Path>) -> Result<()> {
    let cfg = project_root
        .map(|p| Config::load(&p.join("unison.toml")).ok())
        .flatten();

    let (check_web, check_ios, check_android, check_ts) = match &cfg {
        Some(c) => (c.platforms.web, c.platforms.ios, c.platforms.android, matches!(c.project.lang, Lang::Ts)),
        None => (true, true, true, true),
    };

    let mut checks = Vec::new();
    checks.push(which("cargo", "install Rust via https://rustup.rs"));
    checks.push(which("rustup", "install Rust via https://rustup.rs"));

    if check_web {
        checks.push(which("trunk", "cargo install trunk"));
        checks.push(which("wasm-bindgen", "cargo install wasm-bindgen-cli"));
        checks.push(rustup_target("wasm32-unknown-unknown"));
    }
    if check_ios {
        checks.push(which("xcodebuild", "install Xcode from the App Store (macOS only)"));
    }
    if check_android {
        checks.push(which("cargo-ndk", "cargo install cargo-ndk"));
        checks.push(env_set("ANDROID_HOME", "install Android SDK and set ANDROID_HOME"));
    }
    if check_ts {
        checks.push(which("node", "install Node.js from https://nodejs.org"));
        checks.push(which("npx", "install Node.js from https://nodejs.org"));
    }

    let mut all_ok = true;
    for c in &checks {
        if c.present {
            println!("[OK] {}", c.name);
        } else {
            all_ok = false;
            println!("[MISSING] {} — install via: {}", c.name, c.install_hint);
        }
    }

    if !all_ok {
        std::process::exit(1);
    }
    Ok(())
}

fn which(name: &'static str, install_hint: &'static str) -> Check {
    let present = Command::new(name).arg("--version").output().is_ok();
    Check { name, present, install_hint }
}

fn rustup_target(target: &'static str) -> Check {
    let out = Command::new("rustup").args(["target", "list", "--installed"]).output();
    let present = matches!(out, Ok(o) if String::from_utf8_lossy(&o.stdout).lines().any(|l| l.trim() == target));
    Check {
        name: target,
        present,
        install_hint: "rustup target add <name>",
    }
}

fn env_set(var: &'static str, install_hint: &'static str) -> Check {
    Check {
        name: var,
        present: std::env::var(var).is_ok(),
        install_hint,
    }
}
