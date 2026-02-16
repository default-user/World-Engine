use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use worldspace_common::EntityId;
use worldspace_kernel::{EntityData, World, WorldEvent};

/// A content-addressed snapshot of the world state at a specific tick.
///
/// The hash is computed from the canonical CBOR serialized world state,
/// enabling corruption detection on load.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// The tick at which this snapshot was taken.
    pub tick: u64,
    /// The seed at snapshot time (for deterministic continuation).
    pub seed: u64,
    /// Serialized entity data keyed by entity id (BTreeMap for deterministic ordering).
    pub entities: BTreeMap<EntityId, EntityData>,
    /// SHA-256 hash for integrity verification (hex encoded).
    pub hash: String,
}

impl Snapshot {
    /// Create a snapshot from the current world state.
    pub fn capture(world: &World) -> Self {
        let entities: BTreeMap<EntityId, EntityData> = world.entities().clone();
        let tick = world.tick();
        let seed = world.seed();

        let hash = Self::compute_hash(tick, seed, &entities);

        Self {
            tick,
            seed,
            entities,
            hash,
        }
    }

    /// Verify the snapshot integrity by recomputing the hash.
    pub fn verify(&self) -> bool {
        let expected = Self::compute_hash(self.tick, self.seed, &self.entities);
        self.hash == expected
    }

    /// Restore a world from this snapshot.
    pub fn restore(&self) -> World {
        let mut world = World::with_seed(self.seed);
        world.set_tick(self.tick);
        for (id, data) in &self.entities {
            world.spawn_with_id(*id, data.transform);
        }
        // Drain events since restore is not an authoring operation.
        world.drain_events();
        world
    }

    fn compute_hash(tick: u64, seed: u64, entities: &BTreeMap<EntityId, EntityData>) -> String {
        let mut hasher = Sha256::new();
        hasher.update(tick.to_le_bytes());
        hasher.update(seed.to_le_bytes());
        // BTreeMap iterates in deterministic order
        for (id, data) in entities {
            hasher.update(id.0.as_bytes());
            hasher.update(data.transform.position.x.to_le_bytes());
            hasher.update(data.transform.position.y.to_le_bytes());
            hasher.update(data.transform.position.z.to_le_bytes());
            hasher.update(data.transform.rotation.x.to_le_bytes());
            hasher.update(data.transform.rotation.y.to_le_bytes());
            hasher.update(data.transform.rotation.z.to_le_bytes());
            hasher.update(data.transform.rotation.w.to_le_bytes());
            hasher.update(data.transform.scale.x.to_le_bytes());
            hasher.update(data.transform.scale.y.to_le_bytes());
            hasher.update(data.transform.scale.z.to_le_bytes());
        }
        format!("{:x}", hasher.finalize())
    }
}

/// Append-only event log for persistence and replay.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventLog {
    events: Vec<WorldEvent>,
}

impl EventLog {
    /// Create an empty event log.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append events to the log. Events are never modified after writing.
    pub fn append(&mut self, events: &[WorldEvent]) {
        self.events.extend_from_slice(events);
    }

    /// Number of events in the log.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether the log is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Read-only access to all events.
    pub fn events(&self) -> &[WorldEvent] {
        &self.events
    }

    /// Replay events after a snapshot to reconstruct world state.
    ///
    /// Applies only events with tick > snapshot.tick, skipping events that
    /// are already captured in the snapshot.
    pub fn replay_from(&self, snapshot: &Snapshot) -> World {
        let mut world = snapshot.restore();
        let mut past_snapshot = false;
        for event in &self.events {
            if let WorldEvent::Stepped { tick, .. } = event {
                if *tick <= snapshot.tick {
                    continue;
                }
                past_snapshot = true;
            }
            if !past_snapshot {
                continue;
            }
            match event {
                WorldEvent::Spawned { id, transform } => {
                    world.spawn_with_id(*id, *transform);
                }
                WorldEvent::Despawned { id, .. } => {
                    world.despawn(*id);
                }
                WorldEvent::TransformUpdated { id, new, .. } => {
                    world.set_transform(*id, *new);
                }
                WorldEvent::Stepped { .. } => {
                    world.step();
                }
            }
        }
        world.drain_events();
        world
    }
}

/// In-memory snapshot store for persistence.
///
/// Useful for testing and as a building block. For file-backed persistence,
/// use `WorldStore`.
#[derive(Debug, Default)]
pub struct SnapshotStore {
    snapshots: Vec<Snapshot>,
    log: EventLog,
}

impl SnapshotStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Take a snapshot of the current world and store it.
    pub fn take_snapshot(&mut self, world: &World) -> usize {
        let snap = Snapshot::capture(world);
        self.snapshots.push(snap);
        self.snapshots.len() - 1
    }

    /// Flush pending events from the world into the log.
    pub fn flush_events(&mut self, world: &mut World) {
        let events = world.drain_events();
        self.log.append(&events);
    }

    /// Number of snapshots stored.
    pub fn snapshot_count(&self) -> usize {
        self.snapshots.len()
    }

    /// Get a snapshot by index.
    pub fn get_snapshot(&self, index: usize) -> Option<&Snapshot> {
        self.snapshots.get(index)
    }

    /// Access the event log.
    pub fn event_log(&self) -> &EventLog {
        &self.log
    }

    /// Rollback to a specific snapshot, discarding events after that point.
    pub fn rollback(&self, snapshot_index: usize) -> Option<World> {
        self.snapshots
            .get(snapshot_index)
            .map(|snap| snap.restore())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldspace_common::Transform;

    #[test]
    fn snapshot_capture_and_verify() {
        let mut world = World::with_seed(42);
        world.spawn(Transform::default());
        world.step();

        let snap = Snapshot::capture(&world);
        assert!(snap.verify());
        assert_eq!(snap.tick, 1);
    }

    #[test]
    fn snapshot_corruption_detected() {
        let mut world = World::new();
        world.spawn(Transform::default());

        let mut snap = Snapshot::capture(&world);
        snap.tick = 999; // corrupt the tick
        assert!(!snap.verify());
    }

    #[test]
    fn snapshot_restore_roundtrip() {
        let mut world = World::with_seed(7);
        let id = world.spawn(Transform::default());
        world.step();
        world.step();

        let snap = Snapshot::capture(&world);
        let restored = snap.restore();

        assert_eq!(restored.tick(), world.tick());
        assert_eq!(restored.seed(), world.seed());
        assert_eq!(restored.entity_count(), world.entity_count());
        assert!(restored.get(id).is_some());
    }

    #[test]
    fn event_log_append_and_read() {
        let mut log = EventLog::new();
        assert!(log.is_empty());

        let events = vec![
            WorldEvent::Spawned {
                id: EntityId::new(),
                transform: Transform::default(),
            },
            WorldEvent::Stepped { tick: 1, seed: 42 },
        ];
        log.append(&events);
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn snapshot_store_take_and_rollback() {
        let mut store = SnapshotStore::new();
        let mut world = World::with_seed(10);
        world.spawn(Transform::default());
        world.step();

        store.take_snapshot(&world);
        assert_eq!(store.snapshot_count(), 1);

        world.spawn(Transform::default());
        world.step();
        assert_eq!(world.entity_count(), 2);

        let rolled_back = store.rollback(0).unwrap();
        assert_eq!(rolled_back.entity_count(), 1);
    }

    #[test]
    fn snapshot_store_flush_events() {
        let mut store = SnapshotStore::new();
        let mut world = World::new();
        world.spawn(Transform::default());
        world.step();

        store.flush_events(&mut world);
        assert_eq!(store.event_log().len(), 2); // spawn + step
        assert!(world.events().is_empty()); // drained
    }
}
