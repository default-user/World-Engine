//! World Kernel: authoritative world state, simulation stepping, deterministic replay hooks.
//!
//! # Invariants
//! - Simulation step is pure with respect to inputs for deterministic mode.
//! - All state mutations flow through explicit operations.

pub mod world;

pub use world::World;
