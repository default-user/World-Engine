use worldspace_common::EntityId;
use worldspace_kernel::World;

/// World inspector for developer tooling.
///
/// Provides read-only queries against the world state for debugging,
/// profiling, and development UI.
pub struct WorldInspector;

impl WorldInspector {
    /// Produce a summary of the world state.
    pub fn summary(world: &World) -> WorldSummary {
        WorldSummary {
            tick: world.tick(),
            seed: world.seed(),
            entity_count: world.entity_count(),
            pending_events: world.events().len(),
        }
    }

    /// Get the transform of a specific entity as a formatted string.
    pub fn inspect_entity(world: &World, id: EntityId) -> Option<EntityInfo> {
        world.get(id).map(|data| {
            let p = data.transform.position;
            let r = data.transform.rotation;
            let s = data.transform.scale;
            EntityInfo {
                id,
                position: [p.x, p.y, p.z],
                rotation: [r.x, r.y, r.z, r.w],
                scale: [s.x, s.y, s.z],
            }
        })
    }

    /// List all entity IDs in the world.
    pub fn list_entities(world: &World) -> Vec<EntityId> {
        world.entities().keys().copied().collect()
    }
}

/// Summary of world state for the inspector.
#[derive(Debug, Clone)]
pub struct WorldSummary {
    pub tick: u64,
    pub seed: u64,
    pub entity_count: usize,
    pub pending_events: usize,
}

impl std::fmt::Display for WorldSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "World: tick={} seed={} entities={} pending_events={}",
            self.tick, self.seed, self.entity_count, self.pending_events
        )
    }
}

/// Detailed info about a single entity.
#[derive(Debug, Clone)]
pub struct EntityInfo {
    pub id: EntityId,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

impl std::fmt::Display for EntityInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Entity [{:.8}] pos=({:.2}, {:.2}, {:.2}) scale=({:.2}, {:.2}, {:.2})",
            &self.id.0.to_string()[..8],
            self.position[0],
            self.position[1],
            self.position[2],
            self.scale[0],
            self.scale[1],
            self.scale[2],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldspace_common::Transform;

    #[test]
    fn summary_empty_world() {
        let world = World::new();
        let summary = WorldInspector::summary(&world);
        assert_eq!(summary.tick, 0);
        assert_eq!(summary.entity_count, 0);
    }

    #[test]
    fn summary_with_entities() {
        let mut world = World::new();
        world.spawn(Transform::default());
        world.spawn(Transform::default());
        world.step();

        let summary = WorldInspector::summary(&world);
        assert_eq!(summary.tick, 1);
        assert_eq!(summary.entity_count, 2);
        assert_eq!(summary.pending_events, 3); // 2 spawns + 1 step
    }

    #[test]
    fn inspect_entity_found() {
        let mut world = World::new();
        let id = world.spawn(Transform {
            position: glam::Vec3::new(1.0, 2.0, 3.0),
            ..Transform::default()
        });

        let info = WorldInspector::inspect_entity(&world, id).unwrap();
        assert_eq!(info.position, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn inspect_entity_not_found() {
        let world = World::new();
        let fake_id = EntityId::new();
        assert!(WorldInspector::inspect_entity(&world, fake_id).is_none());
    }

    #[test]
    fn list_entities() {
        let mut world = World::new();
        let id1 = world.spawn(Transform::default());
        let id2 = world.spawn(Transform::default());

        let ids = WorldInspector::list_entities(&world);
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
    }

    #[test]
    fn summary_display() {
        let world = World::new();
        let summary = WorldInspector::summary(&world);
        let s = format!("{summary}");
        assert!(s.contains("tick=0"));
    }
}
