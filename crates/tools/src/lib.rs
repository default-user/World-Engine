//! Developer Tooling: world inspector, timeline scrubber, profiling hooks, benchmarks.
//!
//! # Invariants
//! - Tools are first-class and tested where possible.

/// Placeholder module. Implementation in M1+.
pub fn crate_info() -> &'static str {
    "worldspace-tools v0.1.0"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_loads() {
        assert!(crate_info().contains("tools"));
    }
}
