//! Persistence: snapshot + event log, rollback, branching edits.
//!
//! # Invariants
//! - Event log is append-only.
//! - Snapshots are content-addressed and verifiable (SHA-256).
//! - Rollback reconstructs prior state via snapshot + log replay.
//! - File-backed persistence uses CBOR + zstd compression with hash chain integrity.
//! - Schema versioning ensures fail-closed on mismatch.

mod snapshot;
pub mod store;

pub use snapshot::{EventLog, Snapshot, SnapshotStore};
pub use store::{StoreError, WorldStore};

pub fn crate_info() -> &'static str {
    "worldspace-persist v0.1.0"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_loads() {
        assert!(crate_info().contains("persist"));
    }
}
