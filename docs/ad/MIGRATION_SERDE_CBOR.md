# Migration: serde_cbor -> maintained CBOR crate

## Status: COMPLETED

serde_cbor was declared as a dependency but never used in source code (persist
crate was a placeholder). The dependency was removed entirely from the workspace
and from worldspace-persist/Cargo.toml. No replacement crate was needed since
no CBOR encode/decode code existed yet.

When persist is implemented in M2, use minicbor or ciborium directly instead of
serde_cbor.

## Why
cargo-deny fails on RUSTSEC-2021-0127 because serde_cbor is unmaintained and has no safe upgrade.

## Where
Was introduced by: worldspace-persist -> worldspace-cli, worldspace-desktop.

## Resolution
- serde_cbor removed from workspace Cargo.toml
- serde_cbor removed from crates/persist/Cargo.toml
- deny.toml ignore for RUSTSEC-2021-0127 removed
- No code changes required (dependency was unused)
