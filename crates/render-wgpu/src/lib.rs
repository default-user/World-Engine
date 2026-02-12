//! wgpu render backend for the world engine.
//!
//! Renders a grid floor and instanced cubes for entities with Renderable components.
//! Camera uses a fly-camera model with WASD + mouse look.
//!
//! # Invariants
//! - Renderer never mutates world state.
//! - Camera motion is NOT part of the deterministic kernel.
//! - Kernel tick is separate from render frame rate.

mod camera;
mod gpu;
mod shaders;

pub use camera::FlyCamera;
pub use gpu::WgpuRenderer;
