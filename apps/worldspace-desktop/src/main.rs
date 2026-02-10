use clap::Parser;
use tracing_subscriber::EnvFilter;
use worldspace_kernel::World;

#[derive(Parser)]
#[command(name = "worldspace-desktop", about = "Worldspace desktop application")]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let filter = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(filter))
        .init();

    tracing::info!("worldspace-desktop starting");

    let mut world = World::new();
    tracing::info!("world created, tick={}", world.tick());

    // Placeholder: step the world a few times to prove the kernel works.
    for _ in 0..10 {
        world.step();
    }
    tracing::info!("world stepped to tick={}", world.tick());

    tracing::info!("worldspace-desktop exiting (no window loop in M0)");
    Ok(())
}
