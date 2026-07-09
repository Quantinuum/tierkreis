/*!
This module defines the [`InMemoryStorage`] struct which implements [`AssetStorage`]
by storing files in concurrent map data structure implemented by [`dashmap::DashMap`].
*/
use dashmap::DashMap;
use futures::{
    FutureExt,
    future::{self, BoxFuture},
};
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

    fn exists(&self, key: &AssetKey) -> BoxFuture<'_, miette::Result<bool>> {
        future::ok(self.store.contains_key(key)).boxed()
    }

    fn save(&self, key: &AssetKey, value: Vec<u8>) -> BoxFuture<'_, miette::Result<()>> {
        self.store.insert(*key, value);
        future::ok(()).boxed()
    }

    fn load(&self, key: &AssetKey) -> BoxFuture<'_, miette::Result<Vec<u8>>> {
        let res = self
            .store
            .get(key)
            .ok_or_else(|| miette!("Asset not found in InMemoryStorage"));
        future::ready(res.map(|x| x.clone())).boxed()
    }
}
