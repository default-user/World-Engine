# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- Rust workspace scaffolding with 8 library crates and 2 application crates.
- `worldspace-common`: shared types (`EntityId`, `Transform`).
- `worldspace-kernel`: world state with spawn/despawn/step and basic tests.
- `worldspace-input`: action enum shared across embodiment modes.
- `worldspace-desktop`: placeholder desktop app with CLI arg parsing.
- `worldspace-cli`: CLI tool with `info` subcommand.
- `xtask`: workspace automation (fmt, clippy, test, build).
- CI workflow with fmt, clippy, and test jobs across platforms.
- Documentation skeleton: MDX site pages, ADR 0001, postdoc writeup outline.
- Quality configs: `rustfmt.toml`, `clippy.toml`, `deny.toml`.
