use glam::Vec3;

/// A high-level action that any embodiment mode (desktop, VR) can produce.
///
/// The kernel and authoring layer consume actions, never raw input events.
/// This ensures Desktop and VR share the same world logic.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Move the camera or avatar by a delta.
    Move(Vec3),
    /// No-op placeholder for future actions.
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
}
