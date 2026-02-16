//! Asset pipeline scaffolding: content-addressed registry, import stubs.
//!
//! Assets are identified by content-addressed hashes. The renderer consumes
//! assets by handle, never by raw file paths.
//!
//! # Layout
//! Assets are stored in the asset registry which can be persisted to disk.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

/// Content-addressed asset ID computed from the asset data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AssetId(pub u64);

/// A minimal mesh representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mesh {
    pub name: String,
    pub vertex_count: u32,
    pub index_count: u32,
}

/// A minimal material representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Material {
    pub name: String,
    pub base_color: [f32; 4],
}

impl Default for Material {
    fn default() -> Self {
        Self {
            name: "default".into(),
            base_color: [0.8, 0.8, 0.8, 1.0],
        }
    }
}

/// An asset entry in the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Asset {
    Mesh(Mesh),
    Material(Material),
}

/// Errors from asset operations.
#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("asset not found: {0:?}")]
    NotFound(AssetId),
    #[error("glTF parse error: {0}")]
    GltfParse(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Content-addressed asset registry.
///
/// Assets are indexed by their content hash. The registry can be persisted
/// to disk as JSON for inspection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssetStore {
    assets: BTreeMap<AssetId, Asset>,
    next_id: u64,
}

impl AssetStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a mesh and return its asset ID.
    pub fn register_mesh(&mut self, mesh: Mesh) -> AssetId {
        let id = self.content_hash(&mesh.name, mesh.vertex_count, mesh.index_count);
        self.assets.insert(id, Asset::Mesh(mesh));
        id
    }

    /// Register a material and return its asset ID.
    pub fn register_material(&mut self, material: Material) -> AssetId {
        let id = self.content_hash_material(&material.name, &material.base_color);
        self.assets.insert(id, Asset::Material(material));
        id
    }

    /// Get an asset by ID.
    pub fn get(&self, id: AssetId) -> Option<&Asset> {
        self.assets.get(&id)
    }

    /// Get a mesh by ID.
    pub fn get_mesh(&self, id: AssetId) -> Option<&Mesh> {
        match self.assets.get(&id) {
            Some(Asset::Mesh(m)) => Some(m),
            _ => None,
        }
    }

    /// Get a material by ID.
    pub fn get_material(&self, id: AssetId) -> Option<&Material> {
        match self.assets.get(&id) {
            Some(Asset::Material(m)) => Some(m),
            _ => None,
        }
    }

    /// Number of registered assets.
    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    /// Import a glTF file (stub).
    ///
    /// Currently reads the glTF JSON metadata and registers placeholder
    /// mesh/material assets. Full vertex data import is a future task.
    pub fn import_gltf(&mut self, path: impl AsRef<Path>) -> Result<Vec<AssetId>, AssetError> {
        let data = std::fs::read_to_string(path.as_ref())?;
        let json: serde_json::Value =
            serde_json::from_str(&data).map_err(|e| AssetError::GltfParse(e.to_string()))?;

        let mut ids = Vec::new();

        // Extract meshes from glTF JSON
        if let Some(meshes) = json.get("meshes").and_then(|m| m.as_array()) {
            for (i, mesh_val) in meshes.iter().enumerate() {
                let name = mesh_val
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("unnamed")
                    .to_string();
                let mesh = Mesh {
                    name: format!("{name}_{i}"),
                    vertex_count: 0, // Stub: real import would parse accessors
                    index_count: 0,
                };
                ids.push(self.register_mesh(mesh));
            }
        }

        // Extract materials from glTF JSON
        if let Some(materials) = json.get("materials").and_then(|m| m.as_array()) {
            for (i, mat_val) in materials.iter().enumerate() {
                let name = mat_val
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("unnamed")
                    .to_string();
                let base_color = mat_val
                    .get("pbrMetallicRoughness")
                    .and_then(|pbr| pbr.get("baseColorFactor"))
                    .and_then(|c| c.as_array())
                    .map(|arr| {
                        let mut color = [0.8f32, 0.8, 0.8, 1.0];
                        for (i, v) in arr.iter().enumerate().take(4) {
                            if let Some(f) = v.as_f64() {
                                color[i] = f as f32;
                            }
                        }
                        color
                    })
                    .unwrap_or([0.8, 0.8, 0.8, 1.0]);

                let material = Material {
                    name: format!("{name}_{i}"),
                    base_color,
                };
                ids.push(self.register_material(material));
            }
        }

        if ids.is_empty() {
            // Register a default mesh and material for minimal glTF files
            let mesh_id = self.register_mesh(Mesh {
                name: "gltf_default".into(),
                vertex_count: 0,
                index_count: 0,
            });
            ids.push(mesh_id);
        }

        Ok(ids)
    }

    /// Register a default unit cube mesh.
    pub fn register_default_cube(&mut self) -> AssetId {
        self.register_mesh(Mesh {
            name: "unit_cube".into(),
            vertex_count: 24,
            index_count: 36,
        })
    }

    /// Register a default material.
    pub fn register_default_material(&mut self) -> AssetId {
        self.register_material(Material::default())
    }

    /// Save the asset registry to a JSON file.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), AssetError> {
        let file = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }

    /// Load an asset registry from a JSON file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, AssetError> {
        let file = std::fs::File::open(path)?;
        let store: Self = serde_json::from_reader(file)?;
        Ok(store)
    }

    fn content_hash(&mut self, name: &str, vertex_count: u32, index_count: u32) -> AssetId {
        let mut hasher = Sha256::new();
        hasher.update(name.as_bytes());
        hasher.update(vertex_count.to_le_bytes());
        hasher.update(index_count.to_le_bytes());
        let result = hasher.finalize();
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&result[..8]);
        AssetId(u64::from_le_bytes(bytes))
    }

    fn content_hash_material(&mut self, name: &str, color: &[f32; 4]) -> AssetId {
        let mut hasher = Sha256::new();
        hasher.update(name.as_bytes());
        for c in color {
            hasher.update(c.to_le_bytes());
        }
        let result = hasher.finalize();
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&result[..8]);
        AssetId(u64::from_le_bytes(bytes))
    }
}

pub fn crate_info() -> &'static str {
    "worldspace-assets v0.1.0"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_mesh() {
        let mut store = AssetStore::new();
        let id = store.register_mesh(Mesh {
            name: "cube".into(),
            vertex_count: 24,
            index_count: 36,
        });
        assert!(store.get_mesh(id).is_some());
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn register_material() {
        let mut store = AssetStore::new();
        let id = store.register_material(Material::default());
        assert!(store.get_material(id).is_some());
    }

    #[test]
    fn content_addressed_dedup() {
        let mut store = AssetStore::new();
        let id1 = store.register_mesh(Mesh {
            name: "cube".into(),
            vertex_count: 24,
            index_count: 36,
        });
        let id2 = store.register_mesh(Mesh {
            name: "cube".into(),
            vertex_count: 24,
            index_count: 36,
        });
        assert_eq!(id1, id2);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn default_cube_and_material() {
        let mut store = AssetStore::new();
        let cube_id = store.register_default_cube();
        let mat_id = store.register_default_material();
        assert!(store.get_mesh(cube_id).is_some());
        assert!(store.get_material(mat_id).is_some());
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn save_and_load() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let mut store = AssetStore::new();
        store.register_default_cube();
        store.register_default_material();
        store.save(tmp.path()).unwrap();

        let loaded = AssetStore::load(tmp.path()).unwrap();
        assert_eq!(loaded.len(), 2);
    }
}
