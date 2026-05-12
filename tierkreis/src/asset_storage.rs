/*!
This module defines the interface and some standard implementations of [`AssetStorage`]
as well as some utility functions and an [`AssetStorageRegistry`] type.
*/

pub mod file;
pub mod inmemory;
pub mod interface;

pub use crate::asset_storage::file::FileAssetStorage;
pub use crate::asset_storage::inmemory::InMemoryStorage;
pub use crate::asset_storage::interface::{AssetKey, AssetKind, AssetSpec, AssetStorage};

use walkdir::WalkDir;
use std::fs;
use std::hash::BuildHasher;
use std::path::{Path, PathBuf};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use miette::miette;

/// [`AssetStorageRegistry`] is sharable mapping of configured [`AssetStorage`] names
/// to various implementations.
///
/// Note that it is possible to have multiple instances of the same [`AssetStorage`]
/// implementation with different names in order to further separate the storage
/// of Assets as required by the user.
pub type AssetStorageRegistry = Arc<RwLock<HashMap<String, Box<dyn AssetStorage>>>>;

/// Load inputs into a `HashMap` from the various [`AssetStorage`] implementations that
/// contain them as described in each [`AssetSpec`].
///
/// # Errors
///
/// Will return Err if the [`AssetStorageRegistry`] cannot be read from or if
/// an [`AssetStorage`] with the specified name does not exist in the registry.
pub fn load_inputs<S: BuildHasher>(
    registry: &AssetStorageRegistry,
    inputs: &HashMap<String, AssetSpec, S>,
) -> miette::Result<HashMap<String, Vec<u8>>> {
    let registry = registry
        .read()
        .map_err(|err| miette!("Failed to read AssetStorageRegistry: {err}"))?;
    inputs
        .iter()
        .map(|(k, v)| {
            let storage_name = &v.storage_name;
            let storage = registry.get(storage_name).ok_or(miette!(
                "Cannot find AssetStorage in AssetStorageRegistry with name: {storage_name}"
            ))?;
            let asset = storage.load(&v.asset_key)?;
            Ok((k.clone(), asset))
        })
        .collect()
}

/// Save outputs into an [`AssetStorage`] in the [`AssetStorageRegistry`] with a given name
/// and return a [`HashMap`] containing output names and the [`AssetSpec`]s that describe
/// where the Assets were saved.
///
/// # Errors
///
/// Will return Err if the [`AssetStorageRegistry`] cannot be read from or if
/// an [`AssetStorage`] with the specified name does not exist in the registry.
pub fn save_outputs<S: BuildHasher>(
    registry: &AssetStorageRegistry,
    storage_name: &str,
    outputs: HashMap<String, Vec<u8>, S>,
) -> miette::Result<HashMap<String, AssetSpec>> {
    let registry = registry
        .read()
        .map_err(|err| miette!("Failed to read AssetStorageRegistry: {err}"))?;
    let storage = registry.get(storage_name).ok_or(miette!(
        "Cannot find AssetStorage in AssetStorageRegistry with name: {storage_name}"
    ))?;

    outputs
        .into_iter()
        .map(|(k, v)| {
            let asset_key = AssetKey::new();
            storage.save(&asset_key, v)?;
            Ok((
                k,
                AssetSpec {
                    kind: storage.kind(),
                    asset_key,
                    storage_name: storage_name.to_string(),
                },
            ))
        })
        .collect()
}

/// Transfer Assets from various [`AssetStorage`] implementations using
/// [`AssetSpec`]s and an [`AssetStorageRegistry`] into a single [`AssetStorage`].
///
/// # Errors
///
/// Will return Err if the [`AssetStorageRegistry`] cannot be read from or if
/// an [`AssetStorage`] with the specified name does not exist in the registry
/// or if the [`AssetSpec`]s provided cannot be retrieved from the [`AssetStorageRegistry`].
pub fn transfer_assets<S: BuildHasher>(
    registry: &AssetStorageRegistry,
    storage_name_to: &str,
    assets_from: &HashMap<String, AssetSpec, S>,
) -> miette::Result<HashMap<String, AssetSpec>> {
    let registry = registry
        .read()
        .map_err(|err| miette!("Failed to read AssetStorageRegistry: {err}"))?;
    let storage_to = registry.get(storage_name_to).ok_or(miette!(
        "Cannot find AssetStorage in AssetStorageRegistry with name: {storage_name_to}"
    ))?;

    assets_from
        .iter()
        .map(|(k, v)| {
            if v.storage_name == storage_name_to {
                Ok((k.clone(), v.clone()))
            } else {
                let storage_name_from = &v.storage_name;
                let storage_from = registry.get(storage_name_from).ok_or(miette!(
                    "Cannot find AssetStorage in AssetStorageRegistry with name: {storage_name_from}"
                ))?;

                let asset = storage_from.load(&v.asset_key)?;

                let asset_key = AssetKey::new();
                storage_to.save(&asset_key, asset)?;
                Ok((
                    k.clone(),
                    AssetSpec {
                        kind: storage_to.kind(),
                        asset_key,
                        storage_name: storage_name_to.to_string(),
                    },
                ))
            }
        })
        .collect()
}

/// Generate a `total` number of [`AssetSpec`]s for use as Task outputs or similar.
///
/// # Errors
///
/// Will return Err if the [`AssetStorageRegistry`] cannot be read from or if
/// an [`AssetStorage`] with the specified name does not exist in the registry.
pub fn reserve_asset_specs(
    registry: &AssetStorageRegistry,
    storage_name: &str,
    total: usize,
) -> miette::Result<Vec<AssetSpec>> {
    let registry = registry
        .read()
        .map_err(|err| miette!("Failed to read AssetStorageRegistry: {err}"))?;
    let storage = registry.get(storage_name).ok_or(miette!(
        "Cannot find AssetStorage in AssetStorageRegistry with name: {storage_name}"
    ))?;

    let mut asset_specs = Vec::new();
    for _ in 0..total {
        let asset_key = AssetKey::new();
        asset_specs.push(AssetSpec {
            kind: storage.kind(),
            asset_key,
            storage_name: storage_name.to_string(),
        });
    }
    Ok(asset_specs)
}

/// Load Assets from a checkpoint directory into a FileAssetStorage for testing purposes.
///
/// # Panics
///
/// Panics if the checkpoint directory cannot be read or if the assets cannot be loaded.
#[cfg(test)]

pub fn load_checkpoints_dir(path: &Path, storage_name: &str) -> (FileAssetStorage, HashMap<String,AssetSpec>) {
    let files = WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && e.path().parent().map_or(false,|p| p.ends_with("outputs")))
        .map(|e| e.path().to_path_buf())
        .collect::<Vec<PathBuf>>();
    let storage = FileAssetStorage::new(path);
    let mut assets = HashMap::new();
    for file in files {
        let asset_path = file.components().rev().take(3).collect::<PathBuf>().components().rev().collect::<PathBuf>();
        let asset_name = asset_path.to_str().unwrap();
        let asset_key = AssetKey::new();
        let asset: serde_json::Value = serde_json::from_str(fs::read_to_string(&file).unwrap().as_str()).unwrap();
        storage.save(&asset_key, serde_json::to_vec(&asset).unwrap()).unwrap();
        assets.insert(
            asset_name.to_string(),
            AssetSpec {
                kind: storage.kind(),
                asset_key,
                storage_name: storage_name.to_string(),
            },
        );
    }
    (storage, assets)
}

/// Initialize an [`AssetStorageRegistry`] with predefined Assets for use in tests.
///
/// # Panics
///
/// Will panic if the input assets are not dictionaries or they cannot be saved.
#[cfg(test)]
pub fn test_storage_registry(
    assets_for_memory: impl IntoIterator<Item = serde_json::Value>,
    assets_for_files: impl IntoIterator<Item = serde_json::Value>,
) -> (
    AssetStorageRegistry,
    Vec<HashMap<String, AssetSpec>>,
    tempfile::TempDir,
) {
    use tempfile::TempDir;

    use crate::asset_storage::file::FileAssetStorage;
    use crate::asset_storage::inmemory::InMemoryStorage;

    let mut input_asset_sets = Vec::new();

    let memory_storage_name = "memory".to_string();
    let memory_storage = InMemoryStorage::new();

    for input_set in assets_for_memory {
        let inputs: HashMap<String, serde_json::Value> = serde_json::from_value(input_set)
            .expect("Inputs must be convertible to a HashMap<String, Value>");
        let mut input_assets = HashMap::new();

        for (name, value) in inputs {
            let asset_key = AssetKey::new();
            let asset = serde_json::to_vec(&value).unwrap();
            memory_storage.save(&asset_key, asset).unwrap();
            input_assets.insert(
                name,
                AssetSpec {
                    kind: memory_storage.kind(),
                    storage_name: memory_storage_name.clone(),
                    asset_key,
                },
            );
        }

        input_asset_sets.push(input_assets);
    }

    let temp_dir = TempDir::new().unwrap();
    let file_storage_name = "file".to_string();
    let file_storage = FileAssetStorage::new(temp_dir.path());

    for input_set in assets_for_files {
        let inputs: HashMap<String, serde_json::Value> = serde_json::from_value(input_set).unwrap();
        let mut input_assets = HashMap::new();

        for (name, value) in inputs {
            let asset_key = AssetKey::new();
            let asset = serde_json::to_vec(&value).unwrap();
            file_storage.save(&asset_key, asset).unwrap();
            input_assets.insert(
                name,
                AssetSpec {
                    kind: file_storage.kind(),
                    storage_name: file_storage_name.clone(),
                    asset_key,
                },
            );
        }

        input_asset_sets.push(input_assets);
    }

    let mut registry: HashMap<String, Box<dyn AssetStorage>> = HashMap::new();
    registry.insert(memory_storage_name.clone(), Box::new(memory_storage));
    registry.insert(file_storage_name.clone(), Box::new(file_storage));
    let registry = Arc::new(RwLock::new(registry));

    (registry, input_asset_sets, temp_dir)
}

/// Check that an [`AssetStorageRegistry`] contain an expected Asset, for use in tests.
///
/// # Panics
///
/// Will panic if outputs do not match the expected Value.
#[cfg(test)]
pub fn assert_registry_contains_values<S: BuildHasher>(
    registry: &AssetStorageRegistry,
    storage_name: &str,
    outputs: &HashMap<String, AssetSpec, S>,
    expected: serde_json::Value,
) {
    let registry = registry.read().unwrap();
    let storage = registry.get(storage_name).unwrap();

    let expected: HashMap<String, serde_json::Value> =
        serde_json::from_value(expected).expect("Failed to deserialize expected Value.");
    for k in expected.keys() {
        assert!(outputs.contains_key(k), "missing key: {k}");
    }

    for (k, v) in outputs {
        let asset = storage
            .load(&v.asset_key)
            .unwrap_or_else(|err| panic!("Failed to load: {err}"));
        let value: serde_json::Value = serde_json::from_slice(&asset).unwrap();
        assert_eq!(
            value,
            expected
                .get(k)
                .cloned()
                .expect("Failed to extract expected value.")
        );
    }
}
