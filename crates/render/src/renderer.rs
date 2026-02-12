use glam::Vec3;
use worldspace_kernel::World;

/// Camera/view configuration for rendering.
#[derive(Debug, Clone, Copy)]
pub struct RenderView {
    /// Camera position in world space.
    pub eye: Vec3,
    /// Point the camera is looking at.
    pub target: Vec3,
    /// Field of view in degrees.
    pub fov_degrees: f32,
}

impl Default for RenderView {
    fn default() -> Self {
        Self {
            eye: Vec3::new(0.0, 10.0, 10.0),
            target: Vec3::ZERO,
            fov_degrees: 60.0,
        }
    }
}

/// Renderer-agnostic interface. All renderers implement this trait.
///
/// The renderer reads world state and a view configuration, then produces
/// output. It never mutates the world — world truth is kernel-owned.
pub trait Renderer {
    /// The output type produced by this renderer.
    type Output;

    /// Render one frame from the given world state and view.
    fn render(&self, world: &World, view: &RenderView) -> Self::Output;
}

/// Debug text renderer — workaround for the wgpu GPU backend.
///
/// Produces a human-readable string representation of the world state.
/// Useful for CLI output, logging, and testing the render interface.
#[derive(Debug, Default)]
pub struct DebugTextRenderer;

impl DebugTextRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl Renderer for DebugTextRenderer {
    type Output = String;

    fn render(&self, world: &World, view: &RenderView) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "=== World State (tick={}, seed={}) ===\n",
            world.tick(),
            world.seed()
        ));
        out.push_str(&format!("Entities: {}\n", world.entity_count()));
        out.push_str(&format!(
            "Camera: eye=({:.1}, {:.1}, {:.1}) target=({:.1}, {:.1}, {:.1}) fov={:.0}\n",
            view.eye.x,
            view.eye.y,
            view.eye.z,
            view.target.x,
            view.target.y,
            view.target.z,
            view.fov_degrees
        ));

        for (id, data) in world.entities() {
            let p = data.transform.position;
            out.push_str(&format!(
                "  [{:.8}] pos=({:.2}, {:.2}, {:.2})\n",
                &id.0.to_string()[..8],
                p.x,
                p.y,
                p.z
            ));
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldspace_common::Transform;

    #[test]
    fn debug_renderer_empty_world() {
        let world = World::new();
        let renderer = DebugTextRenderer::new();
        let view = RenderView::default();
        let output = renderer.render(&world, &view);

        assert!(output.contains("tick=0"));
        assert!(output.contains("Entities: 0"));
    }

    #[test]
    fn debug_renderer_with_entities() {
        let mut world = World::new();
        world.spawn(Transform::default());
        world.spawn(Transform {
            position: Vec3::new(1.0, 2.0, 3.0),
            ..Transform::default()
        });

        let renderer = DebugTextRenderer::new();
        let view = RenderView::default();
        let output = renderer.render(&world, &view);

        assert!(output.contains("Entities: 2"));
        assert!(output.contains("pos="));
    }

    #[test]
    fn render_view_default() {
        let view = RenderView::default();
        assert_eq!(view.fov_degrees, 60.0);
        assert_eq!(view.target, Vec3::ZERO);
    }
}
