use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use worldspace_common::{EntityId, Transform};

/// An event record produced by every mutation to the world.
///
/// The event log is the foundation for persistence, replay, and undo/redo.
/// Each event captures enough information to reconstruct or reverse the mutation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorldEvent {
    /// Entity was spawned with the given transform.
    Spawned { id: EntityId, transform: Transform },
    /// Entity was despawned. Carries the data it had for undo support.
    Despawned { id: EntityId, transform: Transform },
    /// Entity transform was updated.
    TransformUpdated {
        id: EntityId,
        old: Transform,
        new: Transform,
    },
    /// Simulation advanced one tick with the given seed.
    Stepped { tick: u64, seed: u64 },
}

/// The authoritative world state.
///
/// All mutations go through explicit operations. The kernel owns the truth;
/// renderers, persistence, and authoring tools derive from it.
///
/// Uses BTreeMap for deterministic iteration order across all platforms.
/// Supports deterministic replay via seeded RNG ... given the same seed and
/// sequence of operations, the world will produce identical states.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct World {
    entities: BTreeMap<EntityId, EntityData>,
    tick: u64,
    /// Seed for deterministic RNG. Incremented each step for reproducibility.
    seed: u64,
    /// Append-only event log of all mutations.
    #[serde(skip)]
    event_log: Vec<WorldEvent>,
}

/// Per-entity data stored in the world.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityData {
    pub transform: Transform,
}

impl World {
    /// Create an empty world at tick 0 with seed 0.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a world with a specific seed for deterministic replay.
    pub fn with_seed(seed: u64) -> Self {
        Self {
            seed,
            ..Default::default()
        }
    }

    /// Current simulation tick.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Current RNG seed.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Number of entities in the world.
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Drain and return the event log. Useful for persistence and undo/redo.
    pub fn drain_events(&mut self) -> Vec<WorldEvent> {
        std::mem::take(&mut self.event_log)
    }

    /// Read-only access to the event log.
    pub fn events(&self) -> &[WorldEvent] {
        &self.event_log
    }

    /// Read-only access to all entities (BTreeMap for deterministic iteration).
    pub fn entities(&self) -> &BTreeMap<EntityId, EntityData> {
        &self.entities
    }

    /// Set the tick directly (used for snapshot restore).
    pub fn set_tick(&mut self, tick: u64) {
        self.tick = tick;
    }

    /// Spawn a new entity with the given transform. Returns its id.
    pub fn spawn(&mut self, transform: Transform) -> EntityId {
        let id = EntityId::new();
        self.spawn_with_id(id, transform);
        id
    }

    /// Spawn an entity with a specific id (used for replay/undo).
    pub fn spawn_with_id(&mut self, id: EntityId, transform: Transform) {
        self.entities.insert(id, EntityData { transform });
        self.event_log.push(WorldEvent::Spawned { id, transform });
    }

    /// Remove an entity. Returns the data if it existed.
    pub fn despawn(&mut self, id: EntityId) -> Option<EntityData> {
        let data = self.entities.remove(&id);
        if let Some(ref d) = data {
            self.event_log.push(WorldEvent::Despawned {
                id,
                transform: d.transform,
            });
        }
        data
    }

    /// Get a reference to entity data.
    pub fn get(&self, id: EntityId) -> Option<&EntityData> {
        self.entities.get(&id)
    }

    /// Get a mutable reference to entity data.
    pub fn get_mut(&mut self, id: EntityId) -> Option<&mut EntityData> {
        self.entities.get_mut(&id)
    }

    /// Update an entity's transform and log the change.
    pub fn set_transform(&mut self, id: EntityId, new: Transform) -> bool {
        if let Some(data) = self.entities.get_mut(&id) {
            let old = data.transform;
            data.transform = new;
            self.event_log
                .push(WorldEvent::TransformUpdated { id, old, new });
            true
        } else {
            false
        }
    }

    /// Advance the simulation by one tick.
    ///
    /// Uses a deterministic seed that increments each step. Given the same
    /// starting seed and sequence of operations, replay produces identical states.
    pub fn step(&mut self) {
        self.tick += 1;
        // Deterministic hash: mix the seed using splitmix64 for reproducibility
        // across platforms without depending on floating-point ordering.
        self.seed = splitmix64(self.seed);
        self.event_log.push(WorldEvent::Stepped {
            tick: self.tick,
            seed: self.seed,
        });
    }

    /// Reconstruct world state from a sequence of events (for replay).
    pub fn replay(events: &[WorldEvent]) -> Self {
        let mut world = Self::new();
        for event in events {
            match event {
                WorldEvent::Spawned { id, transform } => {
                    world.entities.insert(
                        *id,
                        EntityData {
                            transform: *transform,
                        },
                    );
                }
                WorldEvent::Despawned { id, .. } => {
                    world.entities.remove(id);
                }
                WorldEvent::TransformUpdated { id, new, .. } => {
                    if let Some(data) = world.entities.get_mut(id) {
                        data.transform = *new;
                    }
                }
                WorldEvent::Stepped { tick, seed } => {
                    world.tick = *tick;
                    world.seed = *seed;
                }
            }
        }
        world
    }

    /// Compute a deterministic hash of the world state for comparison.
    /// Uses canonical (BTreeMap) iteration order.
    pub fn state_hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325; // FNV offset basis
        let mix = |h: &mut u64, bytes: &[u8]| {
            for &b in bytes {
                *h ^= b as u64;
                *h = h.wrapping_mul(0x0100_0000_01b3);
            }
        };
        mix(&mut h, &self.tick.to_le_bytes());
        mix(&mut h, &self.seed.to_le_bytes());
        for (id, data) in &self.entities {
            mix(&mut h, id.0.as_bytes());
            mix(&mut h, &data.transform.position.x.to_le_bytes());
            mix(&mut h, &data.transform.position.y.to_le_bytes());
            mix(&mut h, &data.transform.position.z.to_le_bytes());
            mix(&mut h, &data.transform.rotation.x.to_le_bytes());
            mix(&mut h, &data.transform.rotation.y.to_le_bytes());
            mix(&mut h, &data.transform.rotation.z.to_le_bytes());
            mix(&mut h, &data.transform.rotation.w.to_le_bytes());
            mix(&mut h, &data.transform.scale.x.to_le_bytes());
            mix(&mut h, &data.transform.scale.y.to_le_bytes());
            mix(&mut h, &data.transform.scale.z.to_le_bytes());
        }
        h
    }
}

/// Splitmix64 ... a fast, high-quality deterministic PRNG step function.
/// Used to advance the world seed each tick in a reproducible way.
fn splitmix64(mut state: u64) -> u64 {
    state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
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

    #[test]
    fn deterministic_replay_same_seed() {
        let mut w1 = World::with_seed(42);
        let mut w2 = World::with_seed(42);
        for _ in 0..100 {
            w1.step();
            w2.step();
        }
        assert_eq!(w1.tick(), w2.tick());
        assert_eq!(w1.seed(), w2.seed());
    }

    #[test]
    fn different_seeds_diverge() {
        let mut w1 = World::with_seed(1);
        let mut w2 = World::with_seed(2);
        w1.step();
        w2.step();
        assert_ne!(w1.seed(), w2.seed());
    }

    #[test]
    fn events_are_recorded() {
        let mut w = World::new();
        let id = w.spawn(Transform::default());
        w.step();
        w.despawn(id);
        assert_eq!(w.events().len(), 3); // spawn + step + despawn
    }

    #[test]
    fn drain_events_clears_log() {
        let mut w = World::new();
        w.spawn(Transform::default());
        let events = w.drain_events();
        assert_eq!(events.len(), 1);
        assert!(w.events().is_empty());
    }

    #[test]
    fn set_transform_logs_event() {
        let mut w = World::new();
        let id = w.spawn(Transform::default());
        let new_t = Transform {
            position: glam::Vec3::new(1.0, 2.0, 3.0),
            ..Transform::default()
        };
        assert!(w.set_transform(id, new_t));
        assert_eq!(w.get(id).unwrap().transform.position, new_t.position);
        // spawn + transform update
        assert_eq!(w.events().len(), 2);
    }

    #[test]
    fn replay_reconstructs_state() {
        let mut w = World::with_seed(7);
        let id = w.spawn(Transform::default());
        let moved = Transform {
            position: glam::Vec3::new(5.0, 0.0, 0.0),
            ..Transform::default()
        };
        w.set_transform(id, moved);
        w.step();
        w.step();

        let events = w.events().to_vec();
        let replayed = World::replay(&events);

        assert_eq!(replayed.tick(), w.tick());
        assert_eq!(replayed.entity_count(), w.entity_count());
        assert_eq!(
            replayed.get(id).unwrap().transform.position,
            w.get(id).unwrap().transform.position
        );
    }

    #[test]
    fn state_hash_deterministic() {
        let mut w1 = World::with_seed(42);
        let mut w2 = World::with_seed(42);
        let id1 = w1.spawn(Transform::default());
        w2.spawn_with_id(id1, Transform::default());
        w1.step();
        w2.step();
        assert_eq!(w1.state_hash(), w2.state_hash());
    }

    #[test]
    fn btreemap_gives_deterministic_iteration() {
        let mut w = World::with_seed(0);
        let mut ids = Vec::new();
        for _ in 0..100 {
            ids.push(w.spawn(Transform::default()));
        }
        // BTreeMap iterates in Ord order of EntityId
        let entity_keys: Vec<EntityId> = w.entities().keys().copied().collect();
        let mut sorted = entity_keys.clone();
        sorted.sort();
        assert_eq!(entity_keys, sorted);
    }

    /// Phase I: Determinism boundary – replay_equivalence
    /// Given the same events replayed into a fresh world, the state_hash must match.
    #[test]
    fn replay_equivalence() {
        let mut world = World::with_seed(42);
        let id = world.spawn(Transform::default());
        world.set_transform(
            id,
            Transform {
                position: glam::Vec3::new(5.0, 0.0, 0.0),
                ..Transform::default()
            },
        );
        world.step();
        world.step();

        let events = world.events().to_vec();
        let replayed = World::replay(&events);

        assert_eq!(world.state_hash(), replayed.state_hash());
        assert_eq!(world.tick(), replayed.tick());
        assert_eq!(world.seed(), replayed.seed());
        assert_eq!(world.entity_count(), replayed.entity_count());
    }

    /// Phase I: Determinism boundary – replay with many operations
    #[test]
    fn replay_equivalence_complex() {
        let mut world = World::with_seed(99);
        let mut ids = Vec::new();
        for i in 0..20 {
            let id = world.spawn(Transform {
                position: glam::Vec3::new(i as f32 * 2.0, 0.0, i as f32),
                ..Transform::default()
            });
            ids.push(id);
        }
        // Move some entities
        for i in (0..20).step_by(3) {
            world.set_transform(
                ids[i],
                Transform {
                    position: glam::Vec3::new(100.0, i as f32, 0.0),
                    ..Transform::default()
                },
            );
        }
        // Despawn some
        for i in (1..20).step_by(5) {
            world.despawn(ids[i]);
        }
        // Step several ticks
        for _ in 0..10 {
            world.step();
        }

        let events = world.events().to_vec();
        let replayed = World::replay(&events);
        assert_eq!(world.state_hash(), replayed.state_hash());
    }
}
