use anyhow::Result;
use std::path::Path;

use crate::toolchain::{Invocation, Invoker};

pub struct IosBuildArgs {
    pub release: bool,
    pub profile: bool,
    pub project_name: String,
}

pub fn build(project_root: &Path, invoker: &dyn Invoker, args: IosBuildArgs) -> Result<()> {
    let xcodeproj = format!("platform/ios/{}-ios.xcodeproj", args.project_name);
    let scheme = format!("{}-ios", args.project_name);
    let configuration = if args.release { "Release" } else { "Debug" };

    // Split DerivedData by configuration so back-to-back debug/release builds
    // don't contend on the same `XCBuildData/build.db` lock file.
    let derived_data = format!(
        "target/xcode-derived-{}",
        if args.release { "release" } else { "debug" },
    );

    let mut inv = Invocation::new("xcodebuild", project_root)
        .arg("-project").arg(&xcodeproj)
        .arg("-scheme").arg(&scheme)
        .arg("-configuration").arg(configuration)
        .arg("-derivedDataPath").arg(&derived_data)
        // Without an explicit destination, recent Xcodes pick "My Mac" and
        // fail when the macOS version is newer than Xcode supports. Pin the
        // simulator build to arm64 — the Rust build-phase script only ships
        // `aarch64-apple-ios-sim`, so letting Xcode link x86_64 would fail.
        .arg("-destination").arg("generic/platform=iOS Simulator")
        .arg("ARCHS=arm64")
        .arg("ONLY_ACTIVE_ARCH=YES")
        .arg("build");
    if args.profile {
        inv = inv.env("UNISON_PROFILING", "1");
    }
    let out = invoker.run(&inv.streaming())?;
    if out.status != 0 {
        anyhow::bail!("xcodebuild failed (exit {}) — see output above", out.status);
    }
    Ok(())
}
