# Contributing to World Engine

## Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- `cargo-deny` for license and advisory checks:
  ```bash
  cargo install cargo-deny
  ```

## Dev Quickstart

```bash
# Clone the repo
git clone https://github.com/default-user/World-Engine.git
cd World-Engine

# Build everything
cargo build --workspace

# Run the full check suite (fmt, clippy, test, deny, doc)
cargo run -p xtask -- check

# Run individual checks
cargo run -p xtask -- fmt
cargo run -p xtask -- clippy
cargo run -p xtask -- test
cargo run -p xtask -- deny
cargo run -p xtask -- doc
```

## Repo Layout

| Path | Purpose |
|------|---------|
| `crates/kernel` | Authoritative world state, simulation stepping |
| `crates/persist` | Snapshot + event log, rollback |
| `crates/stream` | World partition, cell streaming, LOD |
| `crates/author` | In-world authoring, undo/redo |
| `crates/render` | Renderer-agnostic interface |
| `crates/input` | Desktop + optional VR input |
| `crates/tools` | Developer tooling, profiling |
| `crates/common` | Shared types and utilities |
| `apps/worldspace-desktop` | Desktop application |
| `apps/worldspace-cli` | CLI operations tool |
| `xtask` | Workspace automation |

## Coding Rules

- No nondeterminism without explicit seeding.
- No renderer/tool mutating world truth.
- Keep PRs under 25 files changed when possible.
- All code must pass `cargo run -p xtask -- check` before submitting a PR.

## PR Workflow

1. Create a feature branch from `main`.
2. Make changes, add tests.
3. Run `cargo run -p xtask -- check` locally.
4. Open a PR targeting `main`.
5. CI must pass before merge.
