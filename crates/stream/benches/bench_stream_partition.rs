use std::hint::black_box;
use std::time::Instant;

use worldspace_common::Transform;
use worldspace_kernel::World;
use worldspace_stream::{CellCoord, GridPartition, StreamConfig, StreamState};

fn make_world(entity_count: usize, spacing: f32) -> World {
    let mut world = World::new();
    let side = (entity_count as f32).sqrt().ceil() as usize;
    for i in 0..entity_count {
        let x = (i % side) as f32 * spacing;
        let z = (i / side) as f32 * spacing;
        world.spawn(Transform {
            position: glam::Vec3::new(x, 0.0, z),
            ..Transform::default()
        });
    }
    world
}

fn bench_rebuild(entity_count: usize, iterations: usize) {
    let world = make_world(entity_count, 4.0);
    let mut grid = GridPartition::new(16.0);

    let start = Instant::now();
    for _ in 0..iterations {
        grid.rebuild(black_box(&world));
    }
    let elapsed = start.elapsed();
    let per_iter = elapsed / iterations as u32;
    println!(
        "  rebuild ({entity_count} entities, {iterations} iters): {per_iter:?}/iter, total {elapsed:?}"
    );
}

fn bench_entities_in_radius(entity_count: usize, radius: i32, iterations: usize) {
    let world = make_world(entity_count, 4.0);
    let mut grid = GridPartition::new(16.0);
    grid.rebuild(&world);

    let center = CellCoord::new(0, 0);
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = black_box(grid.entities_in_radius(black_box(center), black_box(radius)));
    }
    let elapsed = start.elapsed();
    let per_iter = elapsed / iterations as u32;
    println!(
        "  radius query ({entity_count} entities, r={radius}, {iterations} iters): {per_iter:?}/iter, total {elapsed:?}"
    );
}

fn bench_stream_update(entity_count: usize, iterations: usize) {
    let world = make_world(entity_count, 4.0);
    let mut grid = GridPartition::new(16.0);
    grid.rebuild(&world);

    let config = StreamConfig {
        active_radius: 2,
        preload_radius: 4,
        load_budget: 8,
        unload_budget: 8,
    };
    let mut state = StreamState::new(config);

    let start = Instant::now();
    for i in 0..iterations {
        // Simulate viewer moving
        let viewer = CellCoord::new((i % 10) as i32, 0);
        let _ = black_box(state.update(black_box(viewer), black_box(&grid)));
    }
    let elapsed = start.elapsed();
    let per_iter = elapsed / iterations as u32;
    println!(
        "  stream update ({entity_count} entities, {iterations} iters): {per_iter:?}/iter, total {elapsed:?}"
    );
}

fn main() {
    println!("=== Stream Partition Benchmarks ===\n");

    println!("Grid rebuild:");
    bench_rebuild(100, 1000);
    bench_rebuild(1000, 100);
    bench_rebuild(10000, 10);

    println!("\nRadius query:");
    bench_entities_in_radius(1000, 1, 10000);
    bench_entities_in_radius(1000, 3, 10000);
    bench_entities_in_radius(1000, 5, 1000);

    println!("\nStream update (budgeted load/unload):");
    bench_stream_update(100, 10000);
    bench_stream_update(1000, 1000);
    bench_stream_update(10000, 100);

    println!("\n=== Done ===");
}
