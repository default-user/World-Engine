use worldspace_common::{EntityId, Transform};
use worldspace_kernel::World;

/// An editing command that can be applied to the world and reversed.
///
/// Each command carries enough context to undo itself.
#[derive(Debug, Clone)]
pub enum EditCommand {
    /// Spawn an entity. Undo = despawn it.
    Spawn { id: EntityId, transform: Transform },
    /// Despawn an entity. Undo = re-spawn it with its data.
    Despawn { id: EntityId, transform: Transform },
    /// Move an entity. Undo = restore old transform.
    SetTransform {
        id: EntityId,
        old: Transform,
        new: Transform,
    },
}

impl EditCommand {
    /// Produce the inverse command (for undo).
    pub fn inverse(&self) -> Self {
        match self {
            Self::Spawn { id, transform } => Self::Despawn {
                id: *id,
                transform: *transform,
            },
            Self::Despawn { id, transform } => Self::Spawn {
                id: *id,
                transform: *transform,
            },
            Self::SetTransform { id, old, new } => Self::SetTransform {
                id: *id,
                old: *new,
                new: *old,
            },
        }
    }
}

/// Errors from edit operations.
#[derive(Debug, thiserror::Error)]
pub enum EditError {
    #[error("entity {0:?} not found")]
    EntityNotFound(EntityId),
}

/// Editor with undo/redo support for non-destructive world authoring.
///
/// Wraps a `World` and tracks all edit operations in undo/redo stacks.
/// Every authoring operation is reversible via `undo()` and re-applicable via `redo()`.
pub struct Editor {
    undo_stack: Vec<EditCommand>,
    redo_stack: Vec<EditCommand>,
}

impl Editor {
    /// Create a new editor.
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Spawn an entity and push to undo stack.
    pub fn spawn(&mut self, world: &mut World, transform: Transform) -> EntityId {
        let id = world.spawn(transform);
        self.undo_stack.push(EditCommand::Spawn { id, transform });
        self.redo_stack.clear();
        id
    }

    /// Despawn an entity and push to undo stack.
    pub fn despawn(&mut self, world: &mut World, id: EntityId) -> Result<(), EditError> {
        let data = world.despawn(id).ok_or(EditError::EntityNotFound(id))?;
        self.undo_stack.push(EditCommand::Despawn {
            id,
            transform: data.transform,
        });
        self.redo_stack.clear();
        Ok(())
    }

    /// Set an entity's transform and push to undo stack.
    pub fn set_transform(
        &mut self,
        world: &mut World,
        id: EntityId,
        new: Transform,
    ) -> Result<(), EditError> {
        let old = world
            .get(id)
            .ok_or(EditError::EntityNotFound(id))?
            .transform;
        world.set_transform(id, new);
        self.undo_stack
            .push(EditCommand::SetTransform { id, old, new });
        self.redo_stack.clear();
        Ok(())
    }

    /// Undo the last edit. Returns true if an operation was undone.
    pub fn undo(&mut self, world: &mut World) -> bool {
        let Some(cmd) = self.undo_stack.pop() else {
            return false;
        };
        let inverse = cmd.inverse();
        apply_command(world, &inverse);
        self.redo_stack.push(cmd);
        true
    }

    /// Redo the last undone edit. Returns true if an operation was redone.
    pub fn redo(&mut self, world: &mut World) -> bool {
        let Some(cmd) = self.redo_stack.pop() else {
            return false;
        };
        apply_command(world, &cmd);
        self.undo_stack.push(cmd);
        true
    }

    /// Number of operations on the undo stack.
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Number of operations on the redo stack.
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Whether there are operations that can be undone.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Whether there are operations that can be redone.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

fn apply_command(world: &mut World, cmd: &EditCommand) {
    match cmd {
        EditCommand::Spawn { id, transform } => {
            world.spawn_with_id(*id, *transform);
        }
        EditCommand::Despawn { id, .. } => {
            world.despawn(*id);
        }
        EditCommand::SetTransform { id, new, .. } => {
            world.set_transform(*id, *new);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn spawn_and_undo() {
        let mut world = World::new();
        let mut editor = Editor::new();

        let id = editor.spawn(&mut world, Transform::default());
        assert_eq!(world.entity_count(), 1);

        assert!(editor.undo(&mut world));
        assert_eq!(world.entity_count(), 0);
        assert!(world.get(id).is_none());
    }

    #[test]
    fn spawn_undo_redo() {
        let mut world = World::new();
        let mut editor = Editor::new();

        let id = editor.spawn(&mut world, Transform::default());
        editor.undo(&mut world);
        assert_eq!(world.entity_count(), 0);

        editor.redo(&mut world);
        assert_eq!(world.entity_count(), 1);
        assert!(world.get(id).is_some());
    }

    #[test]
    fn despawn_and_undo() {
        let mut world = World::new();
        let mut editor = Editor::new();

        let id = editor.spawn(&mut world, Transform::default());
        editor.despawn(&mut world, id).unwrap();
        assert_eq!(world.entity_count(), 0);

        editor.undo(&mut world);
        assert_eq!(world.entity_count(), 1);
        assert!(world.get(id).is_some());
    }

    #[test]
    fn set_transform_and_undo() {
        let mut world = World::new();
        let mut editor = Editor::new();

        let id = editor.spawn(&mut world, Transform::default());
        let moved = Transform {
            position: Vec3::new(10.0, 0.0, 0.0),
            ..Transform::default()
        };
        editor.set_transform(&mut world, id, moved).unwrap();
        assert_eq!(world.get(id).unwrap().transform.position, moved.position);

        editor.undo(&mut world);
        assert_eq!(world.get(id).unwrap().transform.position, Vec3::ZERO);
    }

    #[test]
    fn redo_cleared_on_new_edit() {
        let mut world = World::new();
        let mut editor = Editor::new();

        editor.spawn(&mut world, Transform::default());
        editor.undo(&mut world);
        assert!(editor.can_redo());

        // New edit clears redo stack
        editor.spawn(&mut world, Transform::default());
        assert!(!editor.can_redo());
    }

    #[test]
    fn undo_empty_returns_false() {
        let mut world = World::new();
        let mut editor = Editor::new();
        assert!(!editor.undo(&mut world));
    }

    #[test]
    fn redo_empty_returns_false() {
        let mut world = World::new();
        let mut editor = Editor::new();
        assert!(!editor.redo(&mut world));
    }

    #[test]
    fn despawn_nonexistent_returns_error() {
        let mut world = World::new();
        let mut editor = Editor::new();
        let fake_id = EntityId::new();
        assert!(editor.despawn(&mut world, fake_id).is_err());
    }
}
