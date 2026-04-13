use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "unison", version, about = "One-stop CLI for Unison 2D game projects")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Scaffold a new cross-platform game project.
    ///
    /// Generates a Rust workspace with Lua or TypeScript scripting, plus platform
    /// shells for Web, iOS, and Android. Each platform can be skipped with
    /// --no-<platform>.
    New {
        /// Project name (directory + Cargo crate name; hyphens become underscores).
        name: String,
        /// Scripting language: `lua` (default) or `ts` (TypeScript via TSTL).
        #[arg(long, default_value = "lua")]
        lang: String,
        /// Skip generating the iOS platform shell.
        #[arg(long)]
        no_ios: bool,
        /// Skip generating the Android platform shell.
        #[arg(long)]
        no_android: bool,
        /// Skip generating the Web platform shell.
        #[arg(long)]
        no_web: bool,
        /// Skip `git init` in the generated project (default: init is run).
        #[arg(long)]
        no_git: bool,
        /// iOS bundle id / Android application id (default: com.example.<crate_name>).
        #[arg(long)]
        bundle_id: Option<String>,
        /// Override the engine tag the generated project pins against (engine-dev flag).
        #[arg(long)]
        engine_tag: Option<String>,
        /// Use an alternate template (power-user escape hatch). NOT YET IMPLEMENTED.
        #[arg(long)]
        template: Option<String>,
    },
    /// Run the dev loop for a platform.
    ///
    /// Web: starts `trunk serve` with hot reload. iOS / Android: prints instructions
    /// for opening the project in Xcode or Android Studio (native IDEs handle the
    /// run loop for those targets).
    Dev {
        /// Target platform: `web`, `ios`, or `android`.
        platform: String,
    },
    /// Build for one or all platforms.
    ///
    /// Accepts `web`, `ios`, `android`, or `all`. Web uses Trunk; iOS and Android
    /// delegate to `xcodebuild` / Gradle respectively.
    Build {
        /// Target platform: `web`, `ios`, `android`, or `all`.
        platform: String,
        /// Build in release mode (optimizations enabled, debug info stripped).
        #[arg(long)]
        release: bool,
        /// Enable the `unison-scripting/profiling` feature flag for profiler output.
        #[arg(long)]
        profile: bool,
    },
    /// Run game-side tests.
    ///
    /// Executes `cargo test` for all projects and also runs `npm test` for
    /// TypeScript projects where a test script is present.
    Test,
    /// Remove build artifacts.
    ///
    /// Deletes `target/`, `platform/web/dist/`, `platform/android/app/build/`, and
    /// (for TS projects) `project/assets/scripts/`.
    Clean,
    /// Check the environment and report missing toolchain pieces.
    ///
    /// Verifies that required tools (cargo, trunk, wasm-bindgen, Xcode CLI tools,
    /// Android SDK, etc.) are available and prints a pass/fail summary with
    /// install hints.
    Doctor,
    /// Point the project at a local engine checkout.
    ///
    /// Adds a `[patch]` override to the project's `Cargo.toml` so that all engine
    /// crates resolve to a local `unison2d` checkout instead of the published
    /// version. Useful when iterating on the engine alongside a game project.
    Link {
        /// Filesystem path to the local `unison2d` engine checkout.
        path: String,
    },
    /// Undo `link` and restore the published engine version.
    Unlink,
    /// Add or remove a platform shell from an existing project.
    ///
    /// Use this after `unison new` if you initially skipped a platform and want
    /// to add it later, or to remove a platform you no longer need.
    Platform {
        #[command(subcommand)]
        action: PlatformAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum PlatformAction {
    /// Add a platform shell to an existing project.
    Add {
        /// Platform to add: `web`, `ios`, or `android`.
        name: String,
    },
    /// Remove a platform shell from an existing project.
    Remove {
        /// Platform to remove: `web`, `ios`, or `android`.
        name: String,
    },
}
