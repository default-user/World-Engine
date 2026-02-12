use glam::Vec3;
use worldspace_common::EntityId;

/// A high-level action that any embodiment mode (desktop, VR) can produce.
///
/// The kernel and authoring layer consume actions, never raw input events.
/// This ensures Desktop and VR share the same world logic.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Move the camera or avatar by a delta.
    Move(Vec3),
    /// Spawn a new entity at the given position.
    SpawnEntity(Vec3),
    /// Despawn the selected entity.
    DespawnEntity(EntityId),
    /// Select an entity for editing.
    Select(EntityId),
    /// Deselect the current selection.
    Deselect,
    /// Undo the last authoring operation.
    Undo,
    /// Redo the last undone operation.
    Redo,
    /// No-op (used for input mapping that hasn't been bound yet).
    Noop,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_move_is_constructible() {
        let a = Action::Move(Vec3::new(1.0, 0.0, 0.0));
        assert!(matches!(a, Action::Move(_)));
    }

    #[test]
    fn action_spawn_entity() {
        let a = Action::SpawnEntity(Vec3::new(0.0, 5.0, 0.0));
        assert!(matches!(a, Action::SpawnEntity(_)));
    }

    #[test]
    fn action_select_deselect() {
        let id = EntityId::new();
        let a = Action::Select(id);
        assert!(matches!(a, Action::Select(_)));
        let b = Action::Deselect;
        assert!(matches!(b, Action::Deselect));
    }

    #[test]
    fn action_undo_redo() {
        assert!(matches!(Action::Undo, Action::Undo));
        assert!(matches!(Action::Redo, Action::Redo));
    }
}
