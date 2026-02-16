//! File-backed world persistence.
//!
//! Layout inside the store directory:
//! ```text
//! world.meta.json          - metadata and schema versions
//! snapshots/
//!   000001.snapshot.cbor.zst - CBOR+zstd compressed snapshots
//! events/
//!   000001.log.cbor.zst      - CBOR+zstd compressed event log segments
//! integrity/
//!   manifest.json            - hash chain manifest
//! ```

use crate::snapshot::Snapshot;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use worldspace_kernel::{World, WorldEvent};

/// Current schema versions.
const WORLD_SCHEMA_VERSION: u32 = 1;
const EVENT_SCHEMA_VERSION: u32 = 1;

/// Errors from file-backed persistence operations.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("CBOR serialization error: {0}")]
    CborEncode(String),
    #[error("CBOR deserialization error: {0}")]
    CborDecode(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("integrity check failed: expected {expected}, got {actual}")]
    IntegrityMismatch { expected: String, actual: String },
    #[error("schema version mismatch: file has v{file_version}, expected v{expected_version}")]
    SchemaMismatch {
        file_version: u32,
        expected_version: u32,
    },
    #[error("no snapshots found")]
    NoSnapshots,
    #[error("store not initialized")]
    NotInitialized,
}

/// Metadata stored in world.meta.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldMeta {
    pub world_schema_version: u32,
    pub event_schema_version: u32,
    pub snapshot_count: u32,
    pub event_segment_count: u32,
}

/// A single entry in the integrity manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub filename: String,
    pub sha256: String,
    pub prev_hash: Option<String>,
}

/// Integrity manifest tracking all segment hashes in a chain.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IntegrityManifest {
    pub entries: Vec<ManifestEntry>,
}

/// File-backed world store with schema versioning and integrity checking.
pub struct WorldStore {
    root: PathBuf,
    meta: WorldMeta,
    manifest: IntegrityManifest,
}

impl WorldStore {
    /// Open or create a world store at the given path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let root = path.as_ref().to_path_buf();
        std::fs::create_dir_all(root.join("snapshots"))?;
        std::fs::create_dir_all(root.join("events"))?;
        std::fs::create_dir_all(root.join("integrity"))?;

        let meta_path = root.join("world.meta.json");
        let manifest_path = root.join("integrity").join("manifest.json");

        let (meta, manifest) = if meta_path.exists() {
            let meta: WorldMeta = serde_json::from_reader(std::fs::File::open(&meta_path)?)?;
            if meta.world_schema_version != WORLD_SCHEMA_VERSION {
                return Err(StoreError::SchemaMismatch {
                    file_version: meta.world_schema_version,
                    expected_version: WORLD_SCHEMA_VERSION,
                });
            }
            if meta.event_schema_version != EVENT_SCHEMA_VERSION {
                return Err(StoreError::SchemaMismatch {
                    file_version: meta.event_schema_version,
                    expected_version: EVENT_SCHEMA_VERSION,
                });
            }
            let manifest: IntegrityManifest = if manifest_path.exists() {
                serde_json::from_reader(std::fs::File::open(&manifest_path)?)?
            } else {
                IntegrityManifest::default()
            };
            (meta, manifest)
        } else {
            let meta = WorldMeta {
                world_schema_version: WORLD_SCHEMA_VERSION,
                event_schema_version: EVENT_SCHEMA_VERSION,
                snapshot_count: 0,
                event_segment_count: 0,
            };
            let manifest = IntegrityManifest::default();
            // Write initial meta
            serde_json::to_writer_pretty(std::fs::File::create(&meta_path)?, &meta)?;
            serde_json::to_writer_pretty(std::fs::File::create(&manifest_path)?, &manifest)?;
            (meta, manifest)
        };

        Ok(Self {
            root,
            meta,
            manifest,
        })
    }

    /// Load the latest snapshot and replay events to reconstruct the world.
    pub fn load_latest(&self) -> Result<World, StoreError> {
        if self.meta.snapshot_count == 0 {
            return Err(StoreError::NoSnapshots);
        }
        let snap = self.load_snapshot(self.meta.snapshot_count)?;
        if !snap.verify() {
            return Err(StoreError::IntegrityMismatch {
                expected: "valid snapshot hash".into(),
                actual: "snapshot hash mismatch".into(),
            });
        }

        // Replay event segments after the snapshot
        let mut world = snap.restore();
        for seg_idx in 1..=self.meta.event_segment_count {
            let events = self.load_event_segment(seg_idx)?;
            for event in &events {
                match event {
                    WorldEvent::Spawned { id, transform } => {
                        // Only replay events past the snapshot tick
                        if world.tick() < snap.tick {
                            continue;
                        }
                        world.spawn_with_id(*id, *transform);
                    }
                    WorldEvent::Despawned { id, .. } => {
                        world.despawn(*id);
                    }
                    WorldEvent::TransformUpdated { id, new, .. } => {
                        world.set_transform(*id, *new);
                    }
                    WorldEvent::Stepped { tick, seed: _ } => {
                        if *tick <= snap.tick {
                            continue;
                        }
                        world.step();
                    }
                }
            }
        }
        world.drain_events();
        Ok(world)
    }

    /// Append events to the store as a new segment.
    pub fn append_events(&mut self, events: &[WorldEvent]) -> Result<(), StoreError> {
        if events.is_empty() {
            return Ok(());
        }
        self.meta.event_segment_count += 1;
        let seg_idx = self.meta.event_segment_count;
        let filename = format!("{:06}.log.cbor.zst", seg_idx);
        let path = self.root.join("events").join(&filename);

        let cbor_bytes = cbor_serialize(events)?;
        let compressed = zstd_compress(&cbor_bytes)?;

        let hash = sha256_hex(&compressed);
        let prev_hash = self.manifest.entries.last().map(|e| e.sha256.clone());

        std::fs::write(&path, &compressed)?;

        self.manifest.entries.push(ManifestEntry {
            filename,
            sha256: hash,
            prev_hash,
        });

        self.save_meta()?;
        self.save_manifest()?;
        Ok(())
    }

    /// Take a snapshot of the world and write it to disk.
    pub fn take_snapshot(&mut self, world: &World) -> Result<(), StoreError> {
        let snap = Snapshot::capture(world);
        self.meta.snapshot_count += 1;
        let snap_idx = self.meta.snapshot_count;
        let filename = format!("{:06}.snapshot.cbor.zst", snap_idx);
        let path = self.root.join("snapshots").join(&filename);

        let cbor_bytes = cbor_serialize(&snap)?;
        let compressed = zstd_compress(&cbor_bytes)?;

        let hash = sha256_hex(&compressed);
        let prev_hash = self.manifest.entries.last().map(|e| e.sha256.clone());

        std::fs::write(&path, &compressed)?;

        self.manifest.entries.push(ManifestEntry {
            filename,
            sha256: hash,
            prev_hash,
        });

        self.save_meta()?;
        self.save_manifest()?;
        Ok(())
    }

    /// Replay from persistence: load latest snapshot and replay all event segments.
    /// Returns the reconstructed world.
    pub fn replay(&self) -> Result<World, StoreError> {
        self.load_latest()
    }

    /// Verify all integrity hashes in the manifest.
    pub fn verify_integrity(&self) -> Result<(), StoreError> {
        let mut prev_hash: Option<String> = None;
        for entry in &self.manifest.entries {
            // Check chain continuity
            if entry.prev_hash != prev_hash {
                return Err(StoreError::IntegrityMismatch {
                    expected: prev_hash.unwrap_or_else(|| "None".into()),
                    actual: entry
                        .prev_hash
                        .clone()
                        .unwrap_or_else(|| "None".into()),
                });
            }

            // Find the file and verify its hash
            let file_path = if entry.filename.contains("snapshot") {
                self.root.join("snapshots").join(&entry.filename)
            } else {
                self.root.join("events").join(&entry.filename)
            };

            let data = std::fs::read(&file_path)?;
            let actual_hash = sha256_hex(&data);
            if actual_hash != entry.sha256 {
                return Err(StoreError::IntegrityMismatch {
                    expected: entry.sha256.clone(),
                    actual: actual_hash,
                });
            }

            prev_hash = Some(entry.sha256.clone());
        }
        Ok(())
    }

    /// Get the path to the store root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Get the metadata.
    pub fn meta(&self) -> &WorldMeta {
        &self.meta
    }

    fn load_snapshot(&self, index: u32) -> Result<Snapshot, StoreError> {
        let filename = format!("{:06}.snapshot.cbor.zst", index);
        let path = self.root.join("snapshots").join(&filename);
        let compressed = std::fs::read(&path)?;

        // Verify hash against manifest
        self.verify_file_hash(&filename, &compressed)?;

        let cbor_bytes = zstd_decompress(&compressed)?;
        cbor_deserialize(&cbor_bytes)
    }

    fn load_event_segment(&self, index: u32) -> Result<Vec<WorldEvent>, StoreError> {
        let filename = format!("{:06}.log.cbor.zst", index);
        let path = self.root.join("events").join(&filename);
        let compressed = std::fs::read(&path)?;

        self.verify_file_hash(&filename, &compressed)?;

        let cbor_bytes = zstd_decompress(&compressed)?;
        cbor_deserialize(&cbor_bytes)
    }

    fn verify_file_hash(&self, filename: &str, data: &[u8]) -> Result<(), StoreError> {
        let actual = sha256_hex(data);
        for entry in &self.manifest.entries {
            if entry.filename == filename {
                if entry.sha256 != actual {
                    return Err(StoreError::IntegrityMismatch {
                        expected: entry.sha256.clone(),
                        actual,
                    });
                }
                return Ok(());
            }
        }
        // File not in manifest is OK for first-time creation
        Ok(())
    }

    fn save_meta(&self) -> Result<(), StoreError> {
        let path = self.root.join("world.meta.json");
        serde_json::to_writer_pretty(std::fs::File::create(path)?, &self.meta)?;
        Ok(())
    }

    fn save_manifest(&self) -> Result<(), StoreError> {
        let path = self.root.join("integrity").join("manifest.json");
        serde_json::to_writer_pretty(std::fs::File::create(path)?, &self.manifest)?;
        Ok(())
    }
}

fn cbor_serialize<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, StoreError> {
    let mut buf = Vec::new();
    ciborium::into_writer(value, &mut buf).map_err(|e| StoreError::CborEncode(e.to_string()))?;
    Ok(buf)
}

fn cbor_deserialize<T: for<'de> Deserialize<'de>>(data: &[u8]) -> Result<T, StoreError> {
    ciborium::from_reader(data).map_err(|e| StoreError::CborDecode(e.to_string()))
}

fn zstd_compress(data: &[u8]) -> Result<Vec<u8>, StoreError> {
    let mut encoder = zstd::Encoder::new(Vec::new(), 3)?;
    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}

fn zstd_decompress(data: &[u8]) -> Result<Vec<u8>, StoreError> {
    let mut decoder = zstd::Decoder::new(data)?;
    let mut buf = Vec::new();
    decoder.read_to_end(&mut buf)?;
    Ok(buf)
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use worldspace_common::Transform;

    #[test]
    fn store_open_creates_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorldStore::open(tmp.path().join("world_data")).unwrap();
        assert_eq!(store.meta().snapshot_count, 0);
        assert_eq!(store.meta().event_segment_count, 0);
        assert!(store.root().join("snapshots").is_dir());
        assert!(store.root().join("events").is_dir());
        assert!(store.root().join("integrity").is_dir());
    }

    #[test]
    fn store_snapshot_and_load() {
        let tmp = tempfile::tempdir().unwrap();
        let mut store = WorldStore::open(tmp.path().join("world_data")).unwrap();

        let mut world = World::with_seed(42);
        let id = world.spawn(Transform::default());
        world.step();
        world.step();

        store.take_snapshot(&world).unwrap();
        store.append_events(&world.drain_events()).unwrap();

        // Reopen and load
        let store2 = WorldStore::open(tmp.path().join("world_data")).unwrap();
        let loaded = store2.load_latest().unwrap();
        assert_eq!(loaded.tick(), world.tick());
        assert_eq!(loaded.seed(), world.seed());
        assert_eq!(loaded.entity_count(), 1);
        assert!(loaded.get(id).is_some());
    }

    #[test]
    fn store_integrity_verification() {
        let tmp = tempfile::tempdir().unwrap();
        let mut store = WorldStore::open(tmp.path().join("world_data")).unwrap();

        let mut world = World::with_seed(7);
        world.spawn(Transform::default());
        world.step();

        store.take_snapshot(&world).unwrap();
        store.verify_integrity().unwrap();
    }

    #[test]
    fn store_integrity_fail_closed_on_corruption() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("world_data");
        let mut store = WorldStore::open(&path).unwrap();

        let mut world = World::with_seed(7);
        world.spawn(Transform::default());
        world.step();

        store.take_snapshot(&world).unwrap();

        // Corrupt the snapshot file
        let snap_path = path.join("snapshots").join("000001.snapshot.cbor.zst");
        let mut data = std::fs::read(&snap_path).unwrap();
        if let Some(byte) = data.last_mut() {
            *byte ^= 0xff;
        }
        std::fs::write(&snap_path, &data).unwrap();

        // Reopen and verify ... should fail
        let store2 = WorldStore::open(&path).unwrap();
        assert!(store2.verify_integrity().is_err());
    }

    #[test]
    fn store_reopen_preserves_state() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("world_data");

        {
            let mut store = WorldStore::open(&path).unwrap();
            let mut world = World::with_seed(99);
            world.spawn(Transform::default());
            world.step();
            store.take_snapshot(&world).unwrap();
        }

        let store2 = WorldStore::open(&path).unwrap();
        assert_eq!(store2.meta().snapshot_count, 1);
    }

    #[test]
    fn store_schema_version_recorded() {
        let tmp = tempfile::tempdir().unwrap();
        let store = WorldStore::open(tmp.path().join("world_data")).unwrap();
        assert_eq!(store.meta().world_schema_version, WORLD_SCHEMA_VERSION);
        assert_eq!(store.meta().event_schema_version, EVENT_SCHEMA_VERSION);
    }

    /// Phase I: persistence round-trip preserves state_hash
    #[test]
    fn persistence_roundtrip_hash_equivalence() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("world_data");
        let mut store = WorldStore::open(&path).unwrap();

        let mut world = World::with_seed(42);
        let _id1 = world.spawn(Transform::default());
        let _id2 = world.spawn(Transform {
            position: glam::Vec3::new(10.0, 5.0, -3.0),
            ..Transform::default()
        });
        world.step();
        world.step();
        world.step();

        let hash_before = world.state_hash();
        store.take_snapshot(&world).unwrap();

        // Reopen and load
        let store2 = WorldStore::open(&path).unwrap();
        let loaded = store2.load_latest().unwrap();
        assert_eq!(loaded.state_hash(), hash_before);
    }

    /// Phase I: schema version mismatch is fail-closed
    #[test]
    fn schema_mismatch_fail_closed() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("world_data");

        // Create a valid store
        let _store = WorldStore::open(&path).unwrap();

        // Tamper with the meta file to have a wrong version
        let meta_path = path.join("world.meta.json");
        let mut meta: WorldMeta =
            serde_json::from_reader(std::fs::File::open(&meta_path).unwrap()).unwrap();
        meta.world_schema_version = 999;
        serde_json::to_writer_pretty(std::fs::File::create(&meta_path).unwrap(), &meta).unwrap();

        // Must fail to open
        let result = WorldStore::open(&path);
        assert!(result.is_err());
        match result {
            Err(StoreError::SchemaMismatch {
                file_version,
                expected_version,
            }) => {
                assert_eq!(file_version, 999);
                assert_eq!(expected_version, WORLD_SCHEMA_VERSION);
            }
            Err(e) => panic!("expected SchemaMismatch, got: {e}"),
            Ok(_) => panic!("expected error, got Ok"),
        }
    }
}
