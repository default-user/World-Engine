//! Embodiment Modes: desktop input and optional VR input mapped to shared actions.
//!
//! # Invariants
//! - Same action graph for Desktop and VR.
//! - VR feature flag is optional and does not fork world logic.

pub mod action;

pub use action::Action;
