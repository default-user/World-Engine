use std::collections::HashSet;
use std::time::{Duration, Instant};

use crate::grid::{CellCoord, GridPartition};

/// Streaming configuration: controls active and preload radii plus per-frame budgets.
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// Radius (in cells) around the viewer that is fully active (entities ticked + rendered).
    pub active_radius: i32,
    /// Radius (in cells) around the viewer that is preloaded (data in memory, not ticked).
    pub preload_radius: i32,
    /// Maximum number of cells to load per frame.
    pub load_budget: usize,
    /// Maximum number of cells to unload per frame.
    pub unload_budget: usize,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            active_radius: 2,
            preload_radius: 4,
            load_budget: 4,
            unload_budget: 4,
        }
    }
}

/// Tracks which cells are currently loaded and manages load/unload budgets per frame.
pub struct StreamState {
    pub config: StreamConfig,
    loaded_cells: HashSet<CellCoord>,
    stats: StreamStats,
}

/// Per-frame streaming statistics for instrumentation.
#[derive(Debug, Clone, Default)]
pub struct StreamStats {
    pub cells_loaded_this_frame: usize,
    pub cells_unloaded_this_frame: usize,
    pub total_loaded_cells: usize,
    pub frame_time: Duration,
}

impl StreamState {
    pub fn new(config: StreamConfig) -> Self {
        Self {
            config,
            loaded_cells: HashSet::new(),
            stats: StreamStats::default(),
        }
    }

    /// Update streaming state based on the viewer's current cell position.
    /// Returns the cells that were loaded and unloaded this frame.
    /// Respects per-frame load/unload budgets.
    pub fn update(
        &mut self,
        viewer_cell: CellCoord,
        grid: &GridPartition,
    ) -> (Vec<CellCoord>, Vec<CellCoord>) {
        let _span = tracing::info_span!("stream_update").entered();
        let frame_start = Instant::now();

        // Determine desired active + preload cells
        let desired = cells_in_radius(viewer_cell, self.config.preload_radius);

        // Cells to load = desired but not yet loaded
        let to_load: Vec<CellCoord> = desired
            .iter()
            .filter(|c| !self.loaded_cells.contains(c))
            // Only load cells that actually have content
            .filter(|c| !grid.entities_in_cell(**c).is_empty())
            .take(self.config.load_budget)
            .copied()
            .collect();

        // Cells to unload = loaded but no longer desired
        let to_unload: Vec<CellCoord> = self
            .loaded_cells
            .iter()
            .filter(|c| !desired.contains(c))
            .take(self.config.unload_budget)
            .copied()
            .collect();

        for c in &to_load {
            tracing::debug!(?c, "loading cell");
            self.loaded_cells.insert(*c);
        }
        for c in &to_unload {
            tracing::debug!(?c, "unloading cell");
            self.loaded_cells.remove(c);
        }

        self.stats = StreamStats {
            cells_loaded_this_frame: to_load.len(),
            cells_unloaded_this_frame: to_unload.len(),
            total_loaded_cells: self.loaded_cells.len(),
            frame_time: frame_start.elapsed(),
        };

        tracing::trace!(
            loaded = to_load.len(),
            unloaded = to_unload.len(),
            total = self.loaded_cells.len(),
            "stream update complete"
        );

        (to_load, to_unload)
    }

    /// Get the set of currently active cells (within active_radius of the viewer).
    pub fn active_cells(&self, viewer_cell: CellCoord) -> HashSet<CellCoord> {
        let active = cells_in_radius(viewer_cell, self.config.active_radius);
        self.loaded_cells.intersection(&active).copied().collect()
    }

    /// Get all currently loaded cells.
    pub fn loaded_cells(&self) -> &HashSet<CellCoord> {
        &self.loaded_cells
    }

    /// Get statistics from the last update.
    pub fn stats(&self) -> &StreamStats {
        &self.stats
    }

    /// Check if a cell is currently loaded.
    pub fn is_loaded(&self, coord: CellCoord) -> bool {
        self.loaded_cells.contains(&coord)
    }
}

/// Compute all cells within a square radius of a center cell.
fn cells_in_radius(center: CellCoord, radius: i32) -> HashSet<CellCoord> {
    let mut result = HashSet::new();
    for dx in -radius..=radius {
        for dz in -radius..=radius {
            result.insert(CellCoord::new(center.x + dx, center.z + dz));
        }
    }
    result
}

/// Frame time tracker for instrumentation.
#[derive(Debug)]
pub struct FrameTimer {
    history: Vec<Duration>,
    capacity: usize,
    index: usize,
    filled: bool,
}

impl FrameTimer {
    pub fn new(capacity: usize) -> Self {
        Self {
            history: vec![Duration::ZERO; capacity],
            capacity,
            index: 0,
            filled: false,
        }
    }

    pub fn record(&mut self, dt: Duration) {
        self.history[self.index] = dt;
        self.index = (self.index + 1) % self.capacity;
        if self.index == 0 {
            self.filled = true;
        }
    }

    pub fn average(&self) -> Duration {
        let count = if self.filled { self.capacity } else { self.index };
        if count == 0 {
            return Duration::ZERO;
        }
        let total: Duration = self.history[..count].iter().sum();
        total / count as u32
    }

    pub fn max(&self) -> Duration {
        let count = if self.filled { self.capacity } else { self.index };
        self.history[..count]
            .iter()
            .copied()
            .max()
            .unwrap_or(Duration::ZERO)
    }

    pub fn min(&self) -> Duration {
        let count = if self.filled { self.capacity } else { self.index };
        self.history[..count]
            .iter()
            .copied()
            .min()
            .unwrap_or(Duration::ZERO)
    }

    pub fn count(&self) -> usize {
        if self.filled {
            self.capacity
        } else {
            self.index
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldspace_common::Transform;
    use worldspace_kernel::World;

    fn make_world_with_entities(count: usize, spacing: f32) -> World {
        let mut world = World::new();
        for i in 0..count {
            world.spawn(Transform {
                position: glam::Vec3::new(i as f32 * spacing, 0.0, 0.0),
                ..Transform::default()
            });
        }
        world
    }

    #[test]
    fn stream_config_defaults() {
        let config = StreamConfig::default();
        assert_eq!(config.active_radius, 2);
        assert_eq!(config.preload_radius, 4);
        assert_eq!(config.load_budget, 4);
        assert_eq!(config.unload_budget, 4);
    }

    #[test]
    fn stream_loads_cells_within_budget() {
        let world = make_world_with_entities(20, 8.0);
        let mut grid = GridPartition::new(16.0);
        grid.rebuild(&world);

        let config = StreamConfig {
            active_radius: 1,
            preload_radius: 2,
            load_budget: 2,
            unload_budget: 2,
        };
        let mut state = StreamState::new(config);

        let viewer = CellCoord::new(0, 0);
        let (loaded, _unloaded) = state.update(viewer, &grid);

        // Should respect load budget of 2
        assert!(loaded.len() <= 2);
        assert_eq!(state.stats().cells_loaded_this_frame, loaded.len());
    }

    #[test]
    fn stream_unloads_when_viewer_moves() {
        let world = make_world_with_entities(20, 8.0);
        let mut grid = GridPartition::new(16.0);
        grid.rebuild(&world);

        let config = StreamConfig {
            active_radius: 1,
            preload_radius: 1,
            load_budget: 100,
            unload_budget: 100,
        };
        let mut state = StreamState::new(config);

        // Load around origin
        state.update(CellCoord::new(0, 0), &grid);
        let loaded_at_origin = state.loaded_cells().len();
        assert!(loaded_at_origin > 0);

        // Move far away - cells at origin should unload
        let (_loaded, unloaded) = state.update(CellCoord::new(100, 100), &grid);
        assert!(!unloaded.is_empty() || state.loaded_cells().is_empty());
    }

    #[test]
    fn active_cells_subset_of_loaded() {
        let world = make_world_with_entities(10, 8.0);
        let mut grid = GridPartition::new(16.0);
        grid.rebuild(&world);

        let config = StreamConfig {
            active_radius: 1,
            preload_radius: 3,
            load_budget: 100,
            unload_budget: 100,
        };
        let mut state = StreamState::new(config);

        let viewer = CellCoord::new(0, 0);
        state.update(viewer, &grid);

        let active = state.active_cells(viewer);
        for cell in &active {
            assert!(state.is_loaded(*cell));
        }
    }

    #[test]
    fn frame_timer_tracks_history() {
        let mut timer = FrameTimer::new(3);
        timer.record(Duration::from_millis(10));
        timer.record(Duration::from_millis(20));
        timer.record(Duration::from_millis(30));

        assert_eq!(timer.count(), 3);
        assert_eq!(timer.average(), Duration::from_millis(20));
        assert_eq!(timer.max(), Duration::from_millis(30));
        assert_eq!(timer.min(), Duration::from_millis(10));
    }

    #[test]
    fn frame_timer_wraps_around() {
        let mut timer = FrameTimer::new(2);
        timer.record(Duration::from_millis(10));
        timer.record(Duration::from_millis(20));
        timer.record(Duration::from_millis(30)); // overwrites first

        assert_eq!(timer.count(), 2);
        // Should contain 20 and 30
        assert_eq!(timer.average(), Duration::from_millis(25));
    }
}
