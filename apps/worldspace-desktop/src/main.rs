use clap::Parser;
use tracing_subscriber::EnvFilter;
use worldspace_author::Editor;
use worldspace_common::Transform;
use worldspace_kernel::World;
use worldspace_persist::SnapshotStore;
use worldspace_render::{DebugTextRenderer, RenderView, Renderer};
use worldspace_stream::GridPartition;
use worldspace_tools::WorldInspector;

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

    // Create world with deterministic seed
    let mut world = World::with_seed(42);
    tracing::info!(
        "world created, tick={}, seed={}",
        world.tick(),
        world.seed()
    );

    // Set up persistence
    let mut store = SnapshotStore::new();

    // Set up authoring with undo/redo
    let mut editor = Editor::new();

    // Spawn entities via the editor (supports undo)
    let id1 = editor.spawn(&mut world, Transform::default());
    let id2 = editor.spawn(
        &mut world,
        Transform {
            position: glam::Vec3::new(10.0, 0.0, 5.0),
            ..Transform::default()
        },
    );
    tracing::info!("spawned 2 entities via editor");

    // Take a snapshot before stepping
    store.take_snapshot(&world);
    tracing::info!("snapshot taken at tick={}", world.tick());

    // Step the simulation deterministically
    for _ in 0..10 {
        world.step();
    }
    tracing::info!(
        "world stepped to tick={}, seed={}",
        world.tick(),
        world.seed()
    );

    // Move an entity via the editor (undo-able)
    editor
        .set_transform(
            &mut world,
            id1,
            Transform {
                position: glam::Vec3::new(5.0, 1.0, 0.0),
                ..Transform::default()
            },
        )
        .expect("entity should exist");

    // Flush events to persistence log
    store.flush_events(&mut world);
    tracing::info!("flushed {} events to persistence", store.event_log().len());

    // Render using debug text renderer (workaround for wgpu)
    let renderer = DebugTextRenderer::new();
    let view = RenderView::default();
    let frame = renderer.render(&world, &view);
    tracing::info!("render frame:\n{frame}");

    // Build grid partition for streaming
    let mut grid = GridPartition::new(16.0);
    grid.rebuild(&world);
    tracing::info!(
        "grid partition: {} cells, {} placements",
        grid.cell_count(),
        grid.total_placements()
    );

    // Inspect world state via developer tools
    let summary = WorldInspector::summary(&world);
    tracing::info!("{summary}");

    // Demonstrate undo
    editor.undo(&mut world);
    tracing::info!(
        "undo: entity {} transform reverted",
        &id1.0.to_string()[..8]
    );

    // Demonstrate rollback to snapshot
    let rolled_back = store.rollback(0).expect("snapshot 0 should exist");
    tracing::info!(
        "rollback to snapshot 0: entities={}",
        rolled_back.entity_count()
    );

    // Verify snapshot integrity
    let snap = store.get_snapshot(0).unwrap();
    tracing::info!(
        "snapshot integrity: {}",
        if snap.verify() { "OK" } else { "CORRUPT" }
    );

    let _ = (id2, rolled_back);
    tracing::info!("worldspace-desktop exiting");
    Ok(())
}
