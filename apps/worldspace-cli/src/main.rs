use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;
use worldspace_common::Transform;
use worldspace_kernel::World;
use worldspace_persist::{Snapshot, SnapshotStore, WorldStore};

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
    /// Replay a persisted world and print its state hash
    Replay {
        /// Path to world data directory
        #[arg(short, long, default_value = "./world_data")]
        path: String,
        /// Number of ticks to simulate (for demo mode without persisted data)
        #[arg(short, long, default_value = "10")]
        ticks: u64,
        /// RNG seed for demo mode
        #[arg(short, long, default_value = "42")]
        seed: u64,
    },
    /// Demonstrate snapshot and rollback
    Snapshot {
        /// Number of entities to spawn
        #[arg(short, long, default_value = "5")]
        entities: usize,
    },
    /// Verify integrity of a persisted world
    Verify {
        /// Path to world data directory
        #[arg(short, long, default_value = "./world_data")]
        path: String,
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
        Commands::Replay { path, ticks, seed } => {
            match WorldStore::open(&path) {
                Ok(store) => match store.load_latest() {
                    Ok(world) => {
                        let hash = world.state_hash();
                        println!("Replay from {path}:");
                        println!(
                            "  tick={}, seed={}, entities={}",
                            world.tick(),
                            world.seed(),
                            world.entity_count()
                        );
                        println!("  state_hash={hash:#018x}");

                        match store.verify_integrity() {
                            Ok(()) => println!("  integrity=OK"),
                            Err(e) => println!("  integrity=FAILED: {e}"),
                        }
                    }
                    Err(_) => {
                        run_demo_replay(ticks, seed);
                    }
                },
                Err(_) => {
                    run_demo_replay(ticks, seed);
                }
            }
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
                "Snapshot: tick={}, entities={}, hash={}, valid={}",
                snap.tick,
                snap.entities.len(),
                &snap.hash[..16],
                snap.verify()
            );

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
        Commands::Verify { path } => {
            println!("Verifying integrity of {path}...");
            let store = WorldStore::open(&path)?;
            match store.verify_integrity() {
                Ok(()) => {
                    println!("Integrity: OK");
                    let world = store.load_latest()?;
                    println!(
                        "World: tick={}, seed={}, entities={}, hash={:#018x}",
                        world.tick(),
                        world.seed(),
                        world.entity_count(),
                        world.state_hash()
                    );
                }
                Err(e) => {
                    println!("Integrity: FAILED");
                    println!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}

fn run_demo_replay(ticks: u64, seed: u64) {
    println!("Deterministic replay demo: seed={seed}, ticks={ticks}");

    let mut w1 = World::with_seed(seed);
    w1.spawn(Transform::default());
    for _ in 0..ticks {
        w1.step();
    }
    let events = w1.events().to_vec();
    let w2 = World::replay(&events);

    println!(
        "Run 1:  tick={}, seed={}, entities={}, hash={:#018x}",
        w1.tick(),
        w1.seed(),
        w1.entity_count(),
        w1.state_hash()
    );
    println!(
        "Replay: tick={}, seed={}, entities={}, hash={:#018x}",
        w2.tick(),
        w2.seed(),
        w2.entity_count(),
        w2.state_hash()
    );
    println!(
        "Match: {}",
        if w1.state_hash() == w2.state_hash() {
            "OK"
        } else {
            "MISMATCH"
        }
    );
}
