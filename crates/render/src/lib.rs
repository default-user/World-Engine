//! Rendering Adapter: renderer-agnostic interface.
//!
//! # Invariants
//! - Renderer cannot mutate world truth directly.
//! - Render state derives from world state and view.
//!
//! # Workaround
//! Provides a trait-based renderer interface with a debug text renderer as a
//! workaround for the wgpu GPU backend. The trait is stable; swap in a wgpu
//! implementation without changing consumers.

mod renderer;

pub use renderer::{DebugTextRenderer, RenderView, Renderer};

pub fn crate_info() -> &'static str {
    "worldspace-render v0.1.0"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_loads() {
        assert!(crate_info().contains("render"));
    }
}
