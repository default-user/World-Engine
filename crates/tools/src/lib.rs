//! Developer Tooling: world inspector, profiling hooks.
//!
//! # Invariants
//! - Tools are first-class and tested where possible.

mod inspector;

pub use inspector::WorldInspector;

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
