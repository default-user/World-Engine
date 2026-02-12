//! Streaming: world partition, cell streaming, LOD budgets.
//!
//! # Invariants
//! - No frame hitching by design goal; measure and regress.
//! - Cells load/unload without corrupting world truth.
//!
//! # Workaround
//! Implements a simple fixed-size grid partitioning scheme as a workaround for
//! a full LOD and async streaming system. Entities are assigned to cells based
//! on position; cells can be queried by coordinate or radius.

mod budget;
mod grid;

pub use budget::{FrameTimer, StreamConfig, StreamState, StreamStats};
pub use grid::{CellCoord, GridPartition};

pub fn crate_info() -> &'static str {
    "worldspace-stream v0.1.0"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_loads() {
        assert!(crate_info().contains("stream"));
    }
}
