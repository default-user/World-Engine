# World Engine

A desktop-first, persistent "real world space" engine with VR as an optional embodiment mode.
Implemented in Rust.

## Overview

World Engine provides:

- **One world kernel, two embodiment modes.** Desktop is primary. VR is a second input and camera
  modality sharing the same world truth, persistence, and authoring operations.
- **Deterministic simulation** with replay and rollback.
- **Persistent world state** via snapshot + event log.
- **In-world authoring** with undo/redo and non-destructive edits.
- **Renderer-agnostic architecture** with a pluggable adapter (wgpu backend initially).

## Quick Start

```bash
# Build the entire workspace
cargo build --workspace

# Run all tests
cargo test --workspace

# Run the desktop application (placeholder in M0)
cargo run -p worldspace-desktop

# Run the CLI tool
cargo run -p worldspace-cli -- info

# Run workspace automation
cargo run -p xtask -- check
```

## Repository Layout

```
crates/
  kernel/     - Authoritative world state, simulation stepping
  persist/    - Snapshot + event log, rollback
  stream/     - World partition, cell streaming, LOD
  author/     - In-world authoring, undo/redo
  render/     - Renderer-agnostic interface
  input/      - Desktop + optional VR input
  tools/      - Developer tooling, profiling
  common/     - Shared types and utilities
apps/
  worldspace-desktop/  - Desktop application (primary)
  worldspace-cli/      - CLI operations tool
xtask/        - Workspace automation
docs/
  mdx/        - Documentation site (MDX pages)
  md/         - Markdown docs (CHANGELOG, ADR, receipts)
```

## Documentation

- [Architecture](docs/mdx/architecture.mdx)
- [CHANGELOG](docs/md/CHANGELOG.md)
- [Architecture Decision Records](docs/md/ADR/)
- [Milestone Receipts](docs/md/receipts/)

## License

MIT License. See [LICENSE](LICENSE) for details.
