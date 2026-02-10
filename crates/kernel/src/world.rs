use std::collections::HashMap;
use worldspace_common::{EntityId, Transform};

/// The authoritative world state.
///
/// All mutations go through explicit operations. The kernel owns the truth;
/// renderers, persistence, and authoring tools derive from it.
#[derive(Debug, Default)]
pub struct World {
    entities: HashMap<EntityId, EntityData>,
    tick: u64,
}

/// Per-entity data stored in the world.
#[derive(Debug, Clone)]
pub struct EntityData {
    pub transform: Transform,
    // Future: component sparse sets will live here or be referenced from here.
}

impl World {
    /// Create an empty world at tick 0.
    pub fn new() -> Self {
        Self::default()
    }

    /// Current simulation tick.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Number of entities in the world.
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Spawn a new entity with the given transform. Returns its id.
    pub fn spawn(&mut self, transform: Transform) -> EntityId {
        let id = EntityId::new();
        self.entities.insert(id, EntityData { transform });
        id
    }

    /// Remove an entity. Returns the data if it existed.
    pub fn despawn(&mut self, id: EntityId) -> Option<EntityData> {
        self.entities.remove(&id)
    }

    /// Get a reference to entity data.
    pub fn get(&self, id: EntityId) -> Option<&EntityData> {
        self.entities.get(&id)
    }

    /// Get a mutable reference to entity data.
    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut EntityData> {
        self.entities.get_mut(&id)
    }

    /// Advance the simulation by one tick.
    ///
    /// In v0.1 this simply increments the tick counter. Future milestones will
    /// add system scheduling and deterministic stepping with seeded RNG.
    pub fn step(&mut self) {
        self.tick += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_starts_empty() {
        let w = World::new();
        assert_eq!(w.tick(), 0);
        assert_eq!(w.entity_count(), 0);
    }

    #[test]
    fn spawn_and_despawn() {
        let mut w = World::new();
        let id = w.spawn(Transform::default());
        assert_eq!(w.entity_count(), 1);
        assert!(w.get(id).is_some());

        let data = w.despawn(id);
        assert!(data.is_some());
        assert_eq!(w.entity_count(), 0);
    }

    #[test]
    fn step_increments_tick() {
        let mut w = World::new();
        w.step();
        w.step();
        w.step();
        assert_eq!(w.tick(), 3);
    }
}
