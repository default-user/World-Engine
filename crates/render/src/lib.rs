//! Rendering Adapter: renderer-agnostic interface, wgpu backend initially.
//!
//! # Invariants
//! - Renderer cannot mutate world truth directly.
//! - Render state derives from world state and view.

/// Placeholder module. Renderer wiring in M1.
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
