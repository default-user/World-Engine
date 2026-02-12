# World Engine workspace automation
# Usage: just <recipe>

default:
    @just --list

# Format all crates
format:
    cargo fmt --all

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# Run clippy lints
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Run all tests
test:
    cargo test --workspace

# Build the entire workspace
build:
    cargo build --workspace

# Run cargo-deny checks
deny:
    cargo deny --log-level info check -c deny.toml

# Build documentation
docs:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

# Run the desktop application
run:
    cargo run -p worldspace-desktop

# Run the CLI tool
cli *ARGS:
    cargo run -p worldspace-cli -- {{ARGS}}

# Run benchmarks
bench:
    cargo bench --workspace

# Run all checks (format, lint, test, deny, docs)
check: fmt-check lint test deny docs

# Run xtask check (alternative)
xtask-check:
    cargo run -p xtask -- check
