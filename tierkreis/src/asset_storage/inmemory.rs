/*!
This module defines the [`InMemoryStorage`] struct which implements [`AssetStorage`]
by storing files in concurrent map data structure implemented by [`dashmap::DashMap`].
*/
use std::ops::Not;

use dashmap::DashMap;
use futures::{
    FutureExt,
    future::{self, BoxFuture},
};
use miette::miette;

use crate::asset_storage::interface::{AssetData, AssetKey, AssetKind, AssetStorage};

/// [`InMemoryStorage`] is an implementation of [`AssetStorage`] that stores
/// Assets in a concurrent map data structure using [`AssetKey`]s as keys.
#[derive(Debug)]
pub struct InMemoryStorage {
    // DashMap is a concurrent HashMap that lets us avoid locking the entire
    // storage when saving/loading values.
    store: DashMap<AssetKey, AssetData>,
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
    fn reserve(&self, key: &AssetKey) -> BoxFuture<'_, miette::Result<AssetKind>> {
        future::ready(
            self.store
                .contains_key(key)
                .not()
                .then_some(AssetKind::Memory)
                .ok_or_else(|| miette!("Asset does not exist in memory")),
        )
        .boxed()
    }

    fn save(&self, key: &AssetKey, value: AssetData) -> BoxFuture<'_, miette::Result<AssetKind>> {
        self.store.insert(*key, value);
        future::ok(AssetKind::Memory).boxed()
    }

    fn load(&self, key: &AssetKey) -> BoxFuture<'_, miette::Result<AssetData>> {
        let res = self
            .store
            .get(key)
            .ok_or_else(|| miette!("Asset not found in InMemoryStorage"));
        future::ready(res.map(|x| AssetData::clone(&x))).boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn loads_share_the_stored_allocation() {
        let storage = InMemoryStorage::new();
        let key = AssetKey::new();
        let data = AssetData::from(vec![42; 1024 * 1024]);
        let stored_ptr = data.as_ptr();

        storage.save(&key, data).await.unwrap();
        let first = storage.load(&key).await.unwrap();
        let second = storage.load(&key).await.unwrap();

        assert_eq!(first.as_ptr(), stored_ptr);
        assert_eq!(second.as_ptr(), stored_ptr);
    }
}
