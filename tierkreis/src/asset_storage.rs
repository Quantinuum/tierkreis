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

use std::hash::BuildHasher;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use miette::miette;
use serde_json::Value;

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
    inputs: HashMap<String, AssetSpec, S>,
) -> miette::Result<HashMap<String, Value>> {
    let registry = registry
        .read()
        .map_err(|err| miette!("Failed to read AssetStorageRegistry: {err}"))?;
    inputs
        .into_iter()
        .map(|(k, v)| {
            let storage_name = &v.storage_name;
            let storage = registry.get(storage_name).ok_or(miette!(
                "Cannot find AssetStorage in AssetStorageRegistry with name: {storage_name}"
            ))?;
            let asset = storage.load(&v.asset_key)?;
            Ok((k, asset))
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
    outputs: HashMap<String, Value, S>,
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
    assets_from: HashMap<String, AssetSpec, S>,
) -> miette::Result<HashMap<String, AssetSpec>> {
    let registry = registry
        .read()
        .map_err(|err| miette!("Failed to read AssetStorageRegistry: {err}"))?;
    let storage_to = registry.get(storage_name_to).ok_or(miette!(
        "Cannot find AssetStorage in AssetStorageRegistry with name: {storage_name_to}"
    ))?;

    assets_from
        .into_iter()
        .map(|(k, v)| {
            if v.storage_name == storage_name_to {
                Ok((k, v))
            } else {
                let storage_name_from = &v.storage_name;
                let storage_from = registry.get(storage_name_from).ok_or(miette!(
                    "Cannot find AssetStorage in AssetStorageRegistry with name: {storage_name_from}"
                ))?;

                let asset = storage_from.load(&v.asset_key)?;

                let asset_key = AssetKey::new();
                storage_to.save(&asset_key, asset)?;
                Ok((
                    k,
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

/// Initialize an [`AssetStorageRegistry`] with predefined Assets for use in tests.
#[cfg(test)]
pub fn test_storage_registry(
    assets_for_memory: impl IntoIterator<Item = Value>,
    assets_for_files: impl IntoIterator<Item = Value>,
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
        let inputs: HashMap<String, Value> = serde_json::from_value(input_set).unwrap();
        let mut input_assets = HashMap::new();

        for (name, value) in inputs {
            let asset_key = AssetKey::new();
            memory_storage.save(&asset_key, value).unwrap();
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
        let inputs: HashMap<String, Value> = serde_json::from_value(input_set).unwrap();
        let mut input_assets = HashMap::new();

        for (name, value) in inputs {
            let asset_key = AssetKey::new();
            file_storage.save(&asset_key, value).unwrap();
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
#[cfg(test)]
pub fn assert_registry_contains_values(
    registry: &AssetStorageRegistry,
    storage_name: &str,
    outputs: &HashMap<String, AssetSpec>,
    expected: Value,
) {
    let registry = registry.read().unwrap();
    let storage = registry.get(storage_name).unwrap();

    let expected: HashMap<String, Value> =
        serde_json::from_value(expected).expect("Failed to deserialize expected Value.");
    for k in expected.keys() {
        assert!(outputs.contains_key(k), "missing key: {k}");
    }

    for (k, v) in outputs {
        let asset = storage
            .load(&v.asset_key)
            .unwrap_or_else(|err| panic!("Failed to load: {err}"));
        assert_eq!(
            asset,
            expected
                .get(k)
                .cloned()
                .expect("Failed to extract expected value.")
        );
    }
}
