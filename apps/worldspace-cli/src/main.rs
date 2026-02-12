use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;
use worldspace_common::Transform;
use worldspace_kernel::World;
use worldspace_persist::{Snapshot, SnapshotStore};

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
    /// Run a deterministic replay demo
    Replay {
        /// Number of ticks to simulate
        #[arg(short, long, default_value = "10")]
        ticks: u64,
        /// RNG seed for deterministic replay
        #[arg(short, long, default_value = "42")]
        seed: u64,
    },
    /// Demonstrate snapshot and rollback
    Snapshot {
        /// Number of entities to spawn
        #[arg(short, long, default_value = "5")]
        entities: usize,
    },
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
            println!("kernel: tick={}", World::new().tick());
            println!("persist: {}", worldspace_persist::crate_info());
            println!("render: {}", worldspace_render::crate_info());
            println!("author: {}", worldspace_author::crate_info());
            println!("stream: {}", worldspace_stream::crate_info());
            println!("tools: {}", worldspace_tools::crate_info());
            println!("input: {}", worldspace_input::crate_info());
        }
        Commands::Replay { ticks, seed } => {
            println!("Deterministic replay: seed={seed}, ticks={ticks}");

            // Run 1
            let mut w1 = World::with_seed(seed);
            w1.spawn(Transform::default());
            for _ in 0..ticks {
                w1.step();
            }
            let events = w1.events().to_vec();

            // Replay from events
            let w2 = World::replay(&events);

            println!(
                "Run 1: tick={}, seed={}, entities={}",
                w1.tick(),
                w1.seed(),
                w1.entity_count()
            );
            println!(
                "Replay: tick={}, seed={}, entities={}",
                w2.tick(),
                w2.seed(),
                w2.entity_count()
            );
            println!(
                "Match: {}",
                if w1.tick() == w2.tick() && w1.seed() == w2.seed() {
                    "OK"
                } else {
                    "MISMATCH"
                }
            );
        }
        Commands::Snapshot { entities } => {
            println!("Snapshot demo: spawning {entities} entities");

            let mut world = World::with_seed(7);
            for i in 0..entities {
                world.spawn(Transform {
                    position: glam::Vec3::new(i as f32 * 2.0, 0.0, 0.0),
                    ..Transform::default()
                });
            }
            world.step();

            let snap = Snapshot::capture(&world);
            println!(
                "Snapshot: tick={}, entities={}, hash={:#x}, valid={}",
                snap.tick,
                snap.entities.len(),
                snap.hash,
                snap.verify()
            );

            // Modify and rollback
            let mut store = SnapshotStore::new();
            store.take_snapshot(&world);
            world.spawn(Transform::default());
            world.step();
            println!(
                "After modification: tick={}, entities={}",
                world.tick(),
                world.entity_count()
            );

            let rolled_back = store.rollback(0).unwrap();
            println!("After rollback: entities={}", rolled_back.entity_count());
        }
    }

    Ok(())
}
