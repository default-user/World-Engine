# Migration: serde_cbor -> maintained CBOR crate

## Why
cargo-deny fails on RUSTSEC-2021-0127 because serde_cbor is unmaintained and has no safe upgrade.
Stage 1 adds a temporary ignore to unblock CI.
Stage 2 must remove serde_cbor and delete the ignore.

## Where
Introduced by: worldspace-persist -> worldspace-cli, worldspace-desktop.

## Replacement candidates
- minicbor (default choice)
- ciborium

## Success criteria
- serde_cbor absent from Cargo.lock
- deny.toml has no ignore for RUSTSEC-2021-0127
- cargo deny passes
- Persist roundtrip tests pass
