# World Engine

A desktop-first, persistent "real world space" engine with VR as an optional embodiment mode.
Implemented in Rust.

## Overview

World Engine provides:

- **One world kernel, two embodiment modes.** Desktop is primary. VR is a second input and camera
  modality sharing the same world truth, persistence, and authoring operations.
- **Deterministic simulation** with replay and rollback via seeded PRNG (splitmix64).
- **Persistent world state** via CBOR+zstd snapshots, append-only event log, and SHA-256 hash chain integrity.
- **In-world authoring** with undo/redo and non-destructive edits.
- **wgpu + egui desktop editor** with fly camera, instanced rendering, entity inspector, and grid floor.
- **ECS component model** with event-sourced mutations for full replay/undo support.
- **Content-addressed asset pipeline** with mesh/material registration and glTF import stub.
- **Streaming with budgets** — active/preload radius, per-frame load/unload limits, frame-time instrumentation.

## Getting Started

```bash
# Build the entire workspace
cargo build --workspace

# Run all tests
cargo test --workspace

# Run the desktop editor (wgpu + egui)
cargo run -p worldspace-desktop

# Run the CLI tool
cargo run -p worldspace-cli -- info

# Replay from a saved world
cargo run -p worldspace-cli -- replay --path /path/to/world_data

# Verify world store integrity
cargo run -p worldspace-cli -- verify --path /path/to/world_data

# Run workspace automation via justfile
just test
just build
just lint
just bench

# Or via xtask
cargo run -p xtask -- check
```

### Desktop Editor Controls

| Key | Action |
|-----|--------|
| WASD | Move camera |
| Space / Ctrl | Up / Down |
| RMB + Mouse | Look around |
| N | Spawn entity |
| Delete / Backspace | Delete selected entity |
| Ctrl+Z | Undo |
| Ctrl+Y | Redo |
| F5 | Save world |
| F9 | Load world |
| F1 | Toggle inspector |

## Repository Layout

```
crates/
  kernel/      - Authoritative world state, simulation stepping, event sourcing
  persist/     - File-backed snapshots + event log, SHA-256 integrity, schema versioning
  stream/      - World partition grid, cell streaming with budgets, frame instrumentation
  author/      - In-world authoring, undo/redo
  ecs/         - BTreeMap-based ECS component model with event-sourced mutations
  assets/      - Content-addressed asset pipeline (mesh, material, glTF import)
  render/      - Renderer-agnostic interface
  render-wgpu/ - wgpu backend with instanced rendering, WGSL shaders, fly camera
  input/       - Desktop + optional VR input actions
  tools/       - Developer tooling, profiling
  common/      - Shared types and utilities (EntityId, Transform)
apps/
  worldspace-desktop/  - Desktop editor (wgpu + egui)
  worldspace-cli/      - CLI operations tool (info, replay, verify)
xtask/        - Workspace automation
docs/
  mdx/        - Documentation site (MDX pages)
  md/         - Markdown docs (CHANGELOG, ADR, receipts)
```

## Documentation

- [Architecture](docs/mdx/architecture.mdx)
- [Determinism Boundary](docs/mdx/determinism_boundary.mdx)
- [Persistence](docs/mdx/persistence.mdx)
- [Streaming](docs/mdx/streaming.mdx)
- [CHANGELOG](docs/md/CHANGELOG.md)
- [Architecture Decision Records](docs/md/ADR/)
- [Milestone Receipts](docs/md/receipts/)

## Roadmap

- VR embodiment adapter (OpenXR)
- Full glTF asset import pipeline
- Terrain heightfield chunks
- Async cell streaming with background I/O
- Multi-user collaborative editing
- Plugin system for custom components

## License

MIT License. See [LICENSE](LICENSE) for details.
