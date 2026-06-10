/*!
This module defines the [`InMemoryStorage`] struct which implements [`AssetStorage`]
by storing files in concurrent map data structure implemented by [`dashmap::DashMap`].
*/
use dashmap::DashMap;
use miette::miette;

use crate::asset_storage::interface::{AssetKey, AssetKind, AssetStorage};

/// [`InMemoryStorage`] is an implementation of [`AssetStorage`] that stores
/// Assets in a concurrent map data structure using [`AssetKey`]s as keys.
#[derive(Clone, Debug)]
pub struct InMemoryStorage {
    // DashMap is a concurrent HashMap that lets us avoid locking the entire
    // storage when saving/loading values.
    store: DashMap<AssetKey, Vec<u8>>,
}

impl InMemoryStorage {
    /// Create a new [`InMemoryStorage`] backed by a [`dashmap::DashMap`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            store: DashMap::new(),
        }
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetStorage for InMemoryStorage {
    fn kind(&self) -> AssetKind {
        AssetKind::Memory
    }

    fn exists(&self, key: &AssetKey) -> miette::Result<bool> {
        Ok(self.store.contains_key(key))
    }

    fn save(&self, key: &AssetKey, value: Vec<u8>) -> miette::Result<()> {
        self.store.insert(*key, value);
        Ok(())
    }

    fn load(&self, key: &AssetKey) -> miette::Result<Vec<u8>> {
        let value = self
            .store
            .get(key)
            .ok_or_else(|| miette!("Asset not found in InMemoryStorage"))?;
        Ok(value.clone())
    }
}
