use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "worldspace-cli", about = "CLI tool for worldspace operations")]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Print engine version and crate info
    Info,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let filter = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(filter))
        .init();

    match cli.command {
        Commands::Info => {
            println!("worldspace-cli v{}", env!("CARGO_PKG_VERSION"));
            println!("kernel: {}", worldspace_kernel::World::new().tick());
            println!("persist: {}", worldspace_persist::crate_info());
        }
    }

    Ok(())
}
