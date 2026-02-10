//! In-World Authoring: non-destructive editing tools, undo/redo, commit edits.
//!
//! # Invariants
//! - All authoring ops are reversible.
//! - Every authoring op produces an event record.

/// Placeholder module. Implementation in M3.
pub fn crate_info() -> &'static str {
    "worldspace-author v0.1.0"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_loads() {
        assert!(crate_info().contains("author"));
    }
}
