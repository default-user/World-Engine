//! Persistence: snapshot + event log, rollback, branching edits.
//!
//! # Invariants
//! - Event log is append-only.
//! - Snapshots are content-addressed and verifiable.
//! - Rollback reconstructs prior state via snapshot + log replay.
//!
//! # Workaround
//! Uses JSON serialization as a workaround for CBOR (serde_cbor is unmaintained).
//! When a maintained CBOR crate (minicbor or ciborium) is adopted, swap the
//! serialization format without changing the public API.

mod snapshot;

pub use snapshot::{EventLog, Snapshot, SnapshotStore};

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
