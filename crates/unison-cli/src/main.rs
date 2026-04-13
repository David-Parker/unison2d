mod cli;

use clap::Parser;
use cli::{Cli, Command};
use unison_cli::commands;

pub const ENGINE_TAG: &str = concat!("v", env!("CARGO_PKG_VERSION"));
pub const ENGINE_GIT_URL: &str = "https://github.com/David-Parker/unison2d";

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::New { name, lang, no_ios, no_android, no_web, no_git, bundle_id, engine_tag, template } => {
            commands::new::run(commands::new::NewArgs {
                name, lang, no_web, no_ios, no_android, no_git, bundle_id, engine_tag, template,
            }, ENGINE_TAG, ENGINE_GIT_URL)
        }
        Command::Doctor => commands::doctor::run(Some(&std::env::current_dir()?)),
        Command::Build { platform, release, profile } => {
            commands::build::run(&std::env::current_dir()?, commands::build::BuildArgs {
                platform, release, profile,
            })
        }
        _ => {
            println!("Not yet implemented");
            Ok(())
        }
    }
}
