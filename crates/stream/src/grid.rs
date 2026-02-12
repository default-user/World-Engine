use std::collections::{HashMap, HashSet};
use worldspace_common::EntityId;
use worldspace_kernel::World;

/// A 2D cell coordinate in the world grid (ignoring Y axis for partitioning).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellCoord {
    pub x: i32,
    pub z: i32,
}

impl CellCoord {
    pub fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }
}

/// Fixed-size grid partitioning of the world.
///
/// Workaround for the full LOD/async streaming system. Entities are assigned
/// to cells based on their XZ position divided by cell_size. Cells can be
/// queried by coordinate or within a radius of a point.
pub struct GridPartition {
    cell_size: f32,
    cells: HashMap<CellCoord, HashSet<EntityId>>,
}

impl GridPartition {
    /// Create a new grid partition with the given cell size.
    pub fn new(cell_size: f32) -> Self {
        assert!(cell_size > 0.0, "cell_size must be positive");
        Self {
            cell_size,
            cells: HashMap::new(),
        }
    }

    /// Cell size used for this partition.
    pub fn cell_size(&self) -> f32 {
        self.cell_size
    }

    /// Rebuild the entire grid from the current world state.
    pub fn rebuild(&mut self, world: &World) {
        self.cells.clear();
        for (id, data) in world.entities() {
            let coord = self.position_to_cell(data.transform.position);
            self.cells.entry(coord).or_default().insert(*id);
        }
    }

    /// Convert a world position to a cell coordinate.
    pub fn position_to_cell(&self, pos: glam::Vec3) -> CellCoord {
        CellCoord {
            x: (pos.x / self.cell_size).floor() as i32,
            z: (pos.z / self.cell_size).floor() as i32,
        }
    }

    /// Get all entity IDs in a specific cell.
    pub fn entities_in_cell(&self, coord: CellCoord) -> HashSet<EntityId> {
        self.cells.get(&coord).cloned().unwrap_or_default()
    }

    /// Get all entity IDs within a radius (in cells) of a center cell.
    pub fn entities_in_radius(&self, center: CellCoord, radius: i32) -> HashSet<EntityId> {
        let mut result = HashSet::new();
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                let coord = CellCoord::new(center.x + dx, center.z + dz);
                if let Some(entities) = self.cells.get(&coord) {
                    result.extend(entities);
                }
            }
        }
        result
    }

    /// Number of non-empty cells.
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    /// Total number of entity placements across all cells.
    pub fn total_placements(&self) -> usize {
        self.cells.values().map(|s| s.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldspace_common::Transform;

    #[test]
    fn position_to_cell_basic() {
        let grid = GridPartition::new(16.0);
        let coord = grid.position_to_cell(glam::Vec3::new(10.0, 0.0, 10.0));
        assert_eq!(coord, CellCoord::new(0, 0));

        let coord = grid.position_to_cell(glam::Vec3::new(20.0, 0.0, -5.0));
        assert_eq!(coord, CellCoord::new(1, -1));
    }

    #[test]
    fn rebuild_from_world() {
        let mut world = World::new();
        world.spawn(Transform::default()); // at origin → cell (0,0)
        world.spawn(Transform {
            position: glam::Vec3::new(20.0, 0.0, 0.0),
            ..Transform::default()
        }); // → cell (1,0) for cell_size=16

        let mut grid = GridPartition::new(16.0);
        grid.rebuild(&world);

        assert_eq!(grid.cell_count(), 2);
        assert_eq!(grid.total_placements(), 2);
    }

    #[test]
    fn entities_in_cell() {
        let mut world = World::new();
        let id = world.spawn(Transform::default());

        let mut grid = GridPartition::new(16.0);
        grid.rebuild(&world);

        let entities = grid.entities_in_cell(CellCoord::new(0, 0));
        assert!(entities.contains(&id));
    }

    #[test]
    fn entities_in_radius() {
        let mut world = World::new();
        let id1 = world.spawn(Transform::default());
        let id2 = world.spawn(Transform {
            position: glam::Vec3::new(20.0, 0.0, 0.0),
            ..Transform::default()
        });

        let mut grid = GridPartition::new(16.0);
        grid.rebuild(&world);

        let nearby = grid.entities_in_radius(CellCoord::new(0, 0), 1);
        assert!(nearby.contains(&id1));
        assert!(nearby.contains(&id2));

        let far = grid.entities_in_radius(CellCoord::new(10, 10), 0);
        assert!(far.is_empty());
    }

    #[test]
    fn empty_cell_returns_empty_set() {
        let grid = GridPartition::new(16.0);
        assert!(grid.entities_in_cell(CellCoord::new(99, 99)).is_empty());
    }
}
