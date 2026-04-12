use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "unison", version, about = "One-stop CLI for Unison 2D game projects")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Scaffold a new cross-platform game project
    New {
        /// Project name (used as directory name and Cargo crate name)
        name: String,
        /// Scripting language
        #[arg(long, default_value = "lua")]
        lang: String,
        #[arg(long)]
        no_ios: bool,
        #[arg(long)]
        no_android: bool,
        #[arg(long)]
        no_web: bool,
        /// Skip `git init` (default: init)
        #[arg(long)]
        no_git: bool,
        /// iOS bundle id / Android application id (default: com.example.<crate_name>)
        #[arg(long)]
        bundle_id: Option<String>,
        /// Override baked-in engine tag (engine-dev flag)
        #[arg(long)]
        engine_tag: Option<String>,
        /// Use an alternate template (power-user escape hatch) - NOT YET IMPLEMENTED
        #[arg(long)]
        template: Option<String>,
    },
    /// Run the dev loop for a platform
    Dev {
        platform: String,
    },
    /// Build a platform (or `all`)
    Build {
        platform: String,
        #[arg(long)]
        release: bool,
        #[arg(long)]
        profile: bool,
    },
    /// Run game-side tests
    Test,
    /// Remove build artifacts
    Clean,
    /// Check environment and report missing toolchain pieces
    Doctor,
    /// Point the project at a local engine checkout
    Link {
        path: String,
    },
    /// Undo `link`
    Unlink,
    /// Add or remove a platform from an existing project
    Platform {
        #[command(subcommand)]
        action: PlatformAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum PlatformAction {
    Add { name: String },
    Remove { name: String },
}
