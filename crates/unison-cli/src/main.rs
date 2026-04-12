mod cli;

use clap::Parser;
use cli::Cli;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    match cli.command {
        _ => {
            println!("Not yet implemented");
            Ok(())
        }
    }
}
