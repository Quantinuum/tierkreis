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
use std::{collections::HashMap, sync::Arc};

use miette::{Context, IntoDiagnostic, miette};
use tokio::sync::RwLock;

/// [`AssetStorageRegistry`] is sharable mapping of configured [`AssetStorage`] names
/// to various implementations.
///
/// Note that it is possible to have multiple instances of the same [`AssetStorage`]
/// implementation with different names in order to further separate the storage
/// of Assets as required by the user.
pub type AssetStorageRegistry = Arc<RwLock<HashMap<String, Box<dyn AssetStorage>>>>;

/// Load assets into a `HashMap` from the various [`AssetStorage`] implementations that
/// contain them as described in each [`AssetSpec`].
///
/// # Errors
///
/// Will return Err if the [`AssetStorageRegistry`] cannot be read from or if
/// an [`AssetStorage`] with the specified name does not exist in the registry.
pub async fn load_assets<S: BuildHasher>(
    registry: &AssetStorageRegistry,
    assets: &HashMap<String, AssetSpec, S>,
) -> miette::Result<HashMap<String, Vec<u8>>> {
    let registry = registry.read().await;

    let mut loaded = HashMap::new();
    for (k, v) in assets {
        let storage_name = &v.storage_name;
        let storage = registry.get(storage_name).ok_or_else(|| {
            miette!("Cannot find AssetStorage in AssetStorageRegistry with name: {storage_name}")
        })?;
        let asset = storage.load(&v.asset_key).await?;
        loaded.insert(k.clone(), asset);
    }
    Ok(loaded)
}

/// Load a single asset by name from the various [`AssetStorage`] implementations that
/// contain them as described in the corresponding asset [`AssetSpec`].
///
/// # Errors
///
/// Will return Err if the [`AssetStorageRegistry`] cannot be read from or if
/// an [`AssetStorage`] with the specified name does not exist in the registry.
pub async fn load_asset<S: BuildHasher>(
    registry: &AssetStorageRegistry,
    assets: &HashMap<String, AssetSpec, S>,
    name: &str,
) -> miette::Result<Vec<u8>> {
    let registry = registry.read().await;
    let asset_spec = assets
        .get(name)
        .ok_or_else(|| miette!("Failed to find asset with name: `{name}`"))?;

    let storage_name = &asset_spec.storage_name;
    let storage = registry.get(storage_name).ok_or_else(|| {
        miette!("Cannot find AssetStorage in AssetStorageRegistry with name: {storage_name}")
    })?;

    let asset = storage.load(&asset_spec.asset_key).await?;

    Ok(asset)
}

/// Load a single asset by name from the various [`AssetStorage`] implementations that
/// contain them as described in the corresponding asset [`AssetSpec`] and then attempt
/// to deserialize the asset as a json list and then save it as multiple assets.
///
/// The assets will be stored in the same storage implementation as the original asset.
///
/// # Errors
///
/// Will return Err if the [`AssetStorageRegistry`] cannot be read from or if
/// an [`AssetStorage`] with the specified name does not exist in the registry.
pub async fn unfold_asset<S: BuildHasher>(
    registry: &AssetStorageRegistry,
    assets: &HashMap<String, AssetSpec, S>,
    name: &str,
) -> miette::Result<Vec<AssetSpec>> {
    let registry = registry.read().await;
    let asset_spec = assets
        .get(name)
        .ok_or_else(|| miette!("Failed to find asset with name: `{name}`"))?;

    let storage_name = &asset_spec.storage_name;
    let storage = registry.get(storage_name).ok_or_else(|| {
        miette!("Cannot find AssetStorage in AssetStorageRegistry with name: {storage_name}")
    })?;

    let asset = storage.load(&asset_spec.asset_key).await?;

    let asset_value_list: Vec<serde_json::Value> =
        serde_json::from_slice(&asset).map_err(|err| miette!("Could not unfold asset: {err}"))?;

    let mut asset_spec_list = Vec::new();
    for asset_value in asset_value_list {
        let asset_key = AssetKey::new();
        let asset_bytes = serde_json::to_vec(&asset_value).into_diagnostic()?;
        storage.save(&asset_key, asset_bytes).await?;

        asset_spec_list.push(AssetSpec {
            kind: storage.kind(),
            storage_name: storage_name.clone(),
            asset_key,
        });
    }

    Ok(asset_spec_list)
}

/// Save assets into an [`AssetStorage`] in the [`AssetStorageRegistry`] with a given name
/// and return a [`HashMap`] containing output names and the [`AssetSpec`]s that describe
/// where the Assets were saved.
///
/// # Errors
///
/// Will return Err if the [`AssetStorageRegistry`] cannot be read from or if
/// an [`AssetStorage`] with the specified name does not exist in the registry.
pub async fn save_assets<S: BuildHasher>(
    registry: &AssetStorageRegistry,
    storage_name: &str,
    raw_assets: HashMap<String, Vec<u8>, S>,
) -> miette::Result<HashMap<String, AssetSpec>> {
    let registry = registry.read().await;
    let storage = registry.get(storage_name).ok_or_else(|| {
        miette!("Cannot find AssetStorage in AssetStorageRegistry with name: {storage_name}")
    })?;

    let mut asset_specs = HashMap::new();
    for (name, raw_asset) in raw_assets {
        let asset_key = AssetKey::new();
        storage.save(&asset_key, raw_asset).await?;
        asset_specs.insert(
            name,
            AssetSpec {
                kind: storage.kind(),
                storage_name: storage_name.to_string(),
                asset_key,
            },
        );
    }

    Ok(asset_specs)
}

/// Save an asset into an [`AssetStorage`] in the [`AssetStorageRegistry`] with a given name
/// and return a [`AssetSpec`] where the Asset was saved.
///
/// # Errors
///
/// Will return Err if the [`AssetStorageRegistry`] cannot be read from or if
/// an [`AssetStorage`] with the specified name does not exist in the registry.
pub async fn save_asset(
    registry: &AssetStorageRegistry,
    storage_name: &str,
    value: Vec<u8>,
) -> miette::Result<AssetSpec> {
    let registry = registry.read().await;
    let storage = registry.get(storage_name).ok_or_else(|| {
        miette!("Cannot find AssetStorage in AssetStorageRegistry with name: {storage_name}")
    })?;

    let asset_key = AssetKey::new();
    storage.save(&asset_key, value).await?;

    Ok(AssetSpec {
        kind: storage.kind(),
        storage_name: storage_name.to_string(),
        asset_key,
    })
}

/// Load Assets and then combine them into a json encoded list, then save the asset
/// into an [`AssetStorage`] in the [`AssetStorageRegistry`] with a given name
/// and return a [`AssetSpec`] where the Asset was saved.
///
/// # Errors
///
/// Will return Err if the [`AssetStorageRegistry`] cannot be read from or if
/// an [`AssetStorage`] with the specified name does not exist in the registry.
pub async fn fold_assets(
    registry: &AssetStorageRegistry,
    storage_name: &str,
    asset_specs: impl IntoIterator<Item = AssetSpec>,
) -> miette::Result<AssetSpec> {
    let registry = registry.read().await;

    let mut asset_values = Vec::new();
    for asset_spec in asset_specs {
        let storage_name = &asset_spec.storage_name;
        let storage = registry.get(&asset_spec.storage_name).ok_or_else(|| {
            miette!("Cannot find AssetStorage in AssetStorageRegistry with name: {storage_name}")
        })?;

        let asset = storage.load(&asset_spec.asset_key).await?;
        let asset_value: serde_json::Value = serde_json::from_slice(&asset).into_diagnostic()?;

        asset_values.push(asset_value);
    }

    let storage = registry.get(storage_name).ok_or_else(|| {
        miette!("Cannot find AssetStorage in AssetStorageRegistry with name: {storage_name}")
    })?;
    let asset_key = AssetKey::new();
    let asset_bytes = serde_json::to_vec(&asset_values).into_diagnostic()?;
    storage.save(&asset_key, asset_bytes).await?;

    Ok(AssetSpec {
        kind: storage.kind(),
        asset_key,
        storage_name: storage_name.to_string(),
    })
}

/// Transfer Assets from various [`AssetStorage`] implementations using
/// [`AssetSpec`]s and an [`AssetStorageRegistry`] into a single [`AssetStorage`].
///
/// # Errors
///
/// Will return Err if the [`AssetStorageRegistry`] cannot be read from or if
/// an [`AssetStorage`] with the specified name does not exist in the registry
/// or if the [`AssetSpec`]s provided cannot be retrieved from the [`AssetStorageRegistry`].
pub async fn transfer_assets<S: BuildHasher>(
    registry: &AssetStorageRegistry,
    storage_name_to: &str,
    assets_from: &HashMap<String, AssetSpec, S>,
) -> miette::Result<HashMap<String, AssetSpec>> {
    let registry = registry.read().await;
    let storage_to = registry.get(storage_name_to).ok_or_else(|| {
        miette!("Cannot find AssetStorage in AssetStorageRegistry with name: {storage_name_to}")
    })?;

    let mut transferred = HashMap::new();
    for (name, asset_from) in assets_from {
        if asset_from.storage_name == storage_name_to {
            transferred.insert(name.clone(), asset_from.clone());
        } else {
            let storage_name_from = &asset_from.storage_name;
            let storage_from = registry.get(storage_name_from).ok_or_else(|| miette!(
                    "Cannot find AssetStorage in AssetStorageRegistry with name: {storage_name_from}"
                ))?;

            let asset = storage_from
                .load(&asset_from.asset_key)
                .await
                .wrap_err("Failed to load asset")?;

            let asset_key = AssetKey::new();
            storage_to
                .save(&asset_key, asset)
                .await
                .wrap_err("Failed to save asset")?;
            transferred.insert(
                name.clone(),
                AssetSpec {
                    kind: storage_to.kind(),
                    asset_key,
                    storage_name: storage_name_to.to_string(),
                },
            );
        }
    }
    Ok(transferred)
}

/// Generate a `total` number of [`AssetSpec`]s for use as Task outputs or similar.
///
/// # Errors
///
/// Will return Err if the [`AssetStorageRegistry`] cannot be read from or if
/// an [`AssetStorage`] with the specified name does not exist in the registry.
pub async fn reserve_asset_specs(
    registry: &AssetStorageRegistry,
    storage_name: &str,
    total: usize,
) -> miette::Result<Vec<AssetSpec>> {
    let registry = registry.read().await;

    let storage = registry.get(storage_name).ok_or_else(|| {
        miette!("Cannot find AssetStorage in AssetStorageRegistry with name: {storage_name}")
    })?;

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
///
/// # Panics
///
/// Will panic if the input assets are not dictionaries or they cannot be saved.
#[cfg(test)]
pub async fn test_storage_registry(
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
            memory_storage.save(&asset_key, asset).await.unwrap();
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
            file_storage.save(&asset_key, asset).await.unwrap();
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
pub async fn assert_registry_contains_values<S: BuildHasher>(
    registry: &AssetStorageRegistry,
    storage_name: &str,
    outputs: &HashMap<String, AssetSpec, S>,
    expected: serde_json::Value,
) {
    let registry = registry.read().await;
    let storage = registry.get(storage_name).unwrap();

    let expected: HashMap<String, serde_json::Value> =
        serde_json::from_value(expected).expect("Failed to deserialize expected Value.");
    for k in expected.keys() {
        assert!(outputs.contains_key(k), "missing key: {k}");
    }

    for (k, v) in outputs {
        let asset = storage
            .load(&v.asset_key)
            .await
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
