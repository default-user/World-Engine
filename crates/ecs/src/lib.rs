//! Minimal deterministic ECS-style component model.
//!
//! Components are stored in BTreeMap for deterministic iteration order.
//! Each component type has its own storage keyed by EntityId.
//!
//! # Invariants
//! - All component mutations produce events.
//! - Iteration order is deterministic (BTreeMap).
//! - Component storage is independent of entity creation order.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use worldspace_common::EntityId;

/// A handle referencing a mesh asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MeshHandle(pub u64);

/// A handle referencing a material asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MaterialHandle(pub u64);

/// Human-readable name component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Name(pub String);

/// Renderable component: references mesh and material assets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Renderable {
    pub mesh: MeshHandle,
    pub material: MaterialHandle,
}

/// Rigid body stub for future physics integration.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RigidBody {
    pub mass: f32,
    pub is_kinematic: bool,
}

impl Default for RigidBody {
    fn default() -> Self {
        Self {
            mass: 1.0,
            is_kinematic: false,
        }
    }
}

/// Collider stub for future physics integration.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Collider {
    Box { half_extents: [f32; 3] },
    Sphere { radius: f32 },
}

impl Default for Collider {
    fn default() -> Self {
        Self::Box {
            half_extents: [0.5, 0.5, 0.5],
        }
    }
}

/// Events produced by component mutations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentEvent {
    NameAdded { entity: EntityId, name: String },
    NameRemoved { entity: EntityId, name: String },
    NameUpdated { entity: EntityId, old: String, new: String },
    RenderableAdded { entity: EntityId, renderable: Renderable },
    RenderableRemoved { entity: EntityId, renderable: Renderable },
    RenderableUpdated { entity: EntityId, old: Renderable, new: Renderable },
    RigidBodyAdded { entity: EntityId, body: RigidBody },
    RigidBodyRemoved { entity: EntityId, body: RigidBody },
    ColliderAdded { entity: EntityId, collider: Collider },
    ColliderRemoved { entity: EntityId, collider: Collider },
}

/// Deterministic component storage for all component types.
///
/// Uses BTreeMap for canonical iteration order. All mutations produce events.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComponentStore {
    names: BTreeMap<EntityId, Name>,
    renderables: BTreeMap<EntityId, Renderable>,
    rigid_bodies: BTreeMap<EntityId, RigidBody>,
    colliders: BTreeMap<EntityId, Collider>,
    #[serde(skip)]
    events: Vec<ComponentEvent>,
}

impl ComponentStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Drain and return all pending component events.
    pub fn drain_events(&mut self) -> Vec<ComponentEvent> {
        std::mem::take(&mut self.events)
    }

    /// Read-only access to pending events.
    pub fn events(&self) -> &[ComponentEvent] {
        &self.events
    }

    // --- Name ---
    pub fn set_name(&mut self, entity: EntityId, name: String) {
        if let Some(old) = self.names.get(&entity) {
            self.events.push(ComponentEvent::NameUpdated {
                entity,
                old: old.0.clone(),
                new: name.clone(),
            });
        } else {
            self.events.push(ComponentEvent::NameAdded {
                entity,
                name: name.clone(),
            });
        }
        self.names.insert(entity, Name(name));
    }

    pub fn remove_name(&mut self, entity: EntityId) -> Option<Name> {
        let removed = self.names.remove(&entity);
        if let Some(ref n) = removed {
            self.events.push(ComponentEvent::NameRemoved {
                entity,
                name: n.0.clone(),
            });
        }
        removed
    }

    pub fn get_name(&self, entity: EntityId) -> Option<&Name> {
        self.names.get(&entity)
    }

    pub fn names(&self) -> &BTreeMap<EntityId, Name> {
        &self.names
    }

    // --- Renderable ---
    pub fn set_renderable(&mut self, entity: EntityId, renderable: Renderable) {
        if let Some(old) = self.renderables.get(&entity) {
            self.events.push(ComponentEvent::RenderableUpdated {
                entity,
                old: *old,
                new: renderable,
            });
        } else {
            self.events.push(ComponentEvent::RenderableAdded {
                entity,
                renderable,
            });
        }
        self.renderables.insert(entity, renderable);
    }

    pub fn remove_renderable(&mut self, entity: EntityId) -> Option<Renderable> {
        let removed = self.renderables.remove(&entity);
        if let Some(r) = removed {
            self.events.push(ComponentEvent::RenderableRemoved {
                entity,
                renderable: r,
            });
        }
        removed
    }

    pub fn get_renderable(&self, entity: EntityId) -> Option<&Renderable> {
        self.renderables.get(&entity)
    }

    pub fn renderables(&self) -> &BTreeMap<EntityId, Renderable> {
        &self.renderables
    }

    // --- RigidBody ---
    pub fn set_rigid_body(&mut self, entity: EntityId, body: RigidBody) {
        self.events.push(ComponentEvent::RigidBodyAdded {
            entity,
            body,
        });
        self.rigid_bodies.insert(entity, body);
    }

    pub fn remove_rigid_body(&mut self, entity: EntityId) -> Option<RigidBody> {
        let removed = self.rigid_bodies.remove(&entity);
        if let Some(body) = removed {
            self.events.push(ComponentEvent::RigidBodyRemoved { entity, body });
        }
        removed
    }

    pub fn get_rigid_body(&self, entity: EntityId) -> Option<&RigidBody> {
        self.rigid_bodies.get(&entity)
    }

    // --- Collider ---
    pub fn set_collider(&mut self, entity: EntityId, collider: Collider) {
        self.events.push(ComponentEvent::ColliderAdded {
            entity,
            collider,
        });
        self.colliders.insert(entity, collider);
    }

    pub fn remove_collider(&mut self, entity: EntityId) -> Option<Collider> {
        let removed = self.colliders.remove(&entity);
        if let Some(collider) = removed {
            self.events.push(ComponentEvent::ColliderRemoved { entity, collider });
        }
        removed
    }

    pub fn get_collider(&self, entity: EntityId) -> Option<&Collider> {
        self.colliders.get(&entity)
    }

    /// Remove all components for an entity.
    pub fn remove_entity(&mut self, entity: EntityId) {
        self.remove_name(entity);
        self.remove_renderable(entity);
        self.remove_rigid_body(entity);
        self.remove_collider(entity);
    }

    /// Replay a component event (for undo/redo or persistence replay).
    pub fn apply_event(&mut self, event: &ComponentEvent) {
        match event {
            ComponentEvent::NameAdded { entity, name } => {
                self.names.insert(*entity, Name(name.clone()));
            }
            ComponentEvent::NameRemoved { entity, .. } => {
                self.names.remove(entity);
            }
            ComponentEvent::NameUpdated { entity, new, .. } => {
                self.names.insert(*entity, Name(new.clone()));
            }
            ComponentEvent::RenderableAdded { entity, renderable } => {
                self.renderables.insert(*entity, *renderable);
            }
            ComponentEvent::RenderableRemoved { entity, .. } => {
                self.renderables.remove(entity);
            }
            ComponentEvent::RenderableUpdated { entity, new, .. } => {
                self.renderables.insert(*entity, *new);
            }
            ComponentEvent::RigidBodyAdded { entity, body } => {
                self.rigid_bodies.insert(*entity, *body);
            }
            ComponentEvent::RigidBodyRemoved { entity, .. } => {
                self.rigid_bodies.remove(entity);
            }
            ComponentEvent::ColliderAdded { entity, collider } => {
                self.colliders.insert(*entity, *collider);
            }
            ComponentEvent::ColliderRemoved { entity, .. } => {
                self.colliders.remove(entity);
            }
        }
    }
}

pub fn crate_info() -> &'static str {
    "worldspace-ecs v0.1.0"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_add_remove() {
        let mut store = ComponentStore::new();
        let id = EntityId::new();
        store.set_name(id, "Test".into());
        assert_eq!(store.get_name(id).unwrap().0, "Test");

        store.remove_name(id);
        assert!(store.get_name(id).is_none());
        assert_eq!(store.events().len(), 2);
    }

    #[test]
    fn name_update_produces_event() {
        let mut store = ComponentStore::new();
        let id = EntityId::new();
        store.set_name(id, "First".into());
        store.set_name(id, "Second".into());
        assert_eq!(store.get_name(id).unwrap().0, "Second");
        // Add + Update
        assert_eq!(store.events().len(), 2);
    }

    #[test]
    fn renderable_add_remove() {
        let mut store = ComponentStore::new();
        let id = EntityId::new();
        let r = Renderable {
            mesh: MeshHandle(1),
            material: MaterialHandle(2),
        };
        store.set_renderable(id, r);
        assert_eq!(store.get_renderable(id), Some(&r));

        store.remove_renderable(id);
        assert!(store.get_renderable(id).is_none());
    }

    #[test]
    fn remove_entity_clears_all() {
        let mut store = ComponentStore::new();
        let id = EntityId::new();
        store.set_name(id, "Test".into());
        store.set_renderable(
            id,
            Renderable {
                mesh: MeshHandle(0),
                material: MaterialHandle(0),
            },
        );
        store.set_rigid_body(id, RigidBody::default());
        store.set_collider(id, Collider::default());

        store.remove_entity(id);
        assert!(store.get_name(id).is_none());
        assert!(store.get_renderable(id).is_none());
        assert!(store.get_rigid_body(id).is_none());
        assert!(store.get_collider(id).is_none());
    }

    #[test]
    fn deterministic_iteration_order() {
        let mut store = ComponentStore::new();
        let mut ids: Vec<EntityId> = (0..50).map(|_| EntityId::new()).collect();
        for id in &ids {
            store.set_name(*id, format!("entity_{}", id.0));
        }
        ids.sort();
        let stored_keys: Vec<EntityId> = store.names().keys().copied().collect();
        assert_eq!(stored_keys, ids);
    }

    #[test]
    fn apply_event_replay() {
        let mut store = ComponentStore::new();
        let id = EntityId::new();
        let event = ComponentEvent::NameAdded {
            entity: id,
            name: "Replayed".into(),
        };
        store.apply_event(&event);
        assert_eq!(store.get_name(id).unwrap().0, "Replayed");
    }

    #[test]
    fn drain_events() {
        let mut store = ComponentStore::new();
        let id = EntityId::new();
        store.set_name(id, "Test".into());
        let events = store.drain_events();
        assert_eq!(events.len(), 1);
        assert!(store.events().is_empty());
    }
}
