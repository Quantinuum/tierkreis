/*!
This module defines the interface contracts that the various [`AssetStorage`]
implementations must satify.
*/
use std::{fmt::Display, path::PathBuf, time::SystemTime};

use miette::miette;
use serde_json::Value;
use uuid::Uuid;

/// [`AssetKind`] is used to categorize [`AssetSpec`] and [`AssetStorage`] implementations
/// as some [Executor][crate::executor::Executor] implementations may make use of this detail.
///
/// For instance the [`SubprocessExecutor`][crate::executor::SubprocessExecutor] struct requires that
/// Task inputs and outputs are of [`AssetKind::File`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum AssetKind {
    /// An Asset that is stored in memory during Workflow execution.
    ///
    /// These Assets will be lost between runs or if the Workflow server
    /// crashes during execution.
    Memory,
    /// An asset that is stored in a file on disk during Workflow Execution.
    ///
    /// Contains a `root` file path that can be used to contruct the full
    /// path to the asset in the filesystem.
    File {
        /// The root directory for constructing the full path to an Asset
        /// in the filesystem.
        root: PathBuf,
    },
}

/// [`AssetSpec`] describes how an Asset should be stored.
///
/// The Asset may not have been persisted yet depending on the Workflow execution
/// and Executors may wish to reserve [`AssetSpec`]s for Tasks.
#[derive(Clone, Debug, PartialEq)]
pub struct AssetSpec {
    /// The kind of Asset that should be saved.
    pub kind: AssetKind,
    /// The name of the [`AssetStorage`] in an [`super::AssetStorageRegistry`] that the
    /// Asset should be saved with.
    pub storage_name: String,
    /// A unique key for the Asset in the [`AssetStorage`].
    pub asset_key: AssetKey,
}

impl AssetSpec {
    /// Return a filesystem path if the Asset is of [`AssetKind::File`].
    ///
    /// # Errors
    ///
    /// Will return Err if the `kind` field is not of [`AssetKind::File`]
    pub fn path(&self) -> miette::Result<PathBuf> {
        match &self.kind {
            AssetKind::File { root: parent } => Ok(parent.join(self.asset_key.0.to_string())),
            _ => Err(miette!("Cannot build a path from a non-File asset!")),
        }
    }
}

/// [`AssetKey`] is a unique key for storing Assets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetKey(Uuid);

impl AssetKey {
    /// Generate a new [`AssetKey`] using the current system time.
    ///
    /// # Panics
    ///
    /// Will panic if the `SystemTime` cannot be converted into a Timestamp that can
    /// be used for Uuid generation.
    #[must_use]
    pub fn new() -> Self {
        let timestamp = SystemTime::now()
            .try_into()
            .expect("Could not convert SystemTime to a Timestamp for AssetKey generation");
        Self(uuid::Uuid::new_v7(timestamp))
    }
}

impl Default for AssetKey {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for AssetKey {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl TryFrom<&str> for AssetKey {
    type Error = uuid::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(Self(value.parse()?))
    }
}

impl Display for AssetKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// The [`AssetStorage`] defines the minimum methods required for Assets to be stored.
///
/// The interface is essentially a key-value store, keyed by [`AssetKey`]s.
pub trait AssetStorage: Send + Sync {
    /// Retrieve the [`AssetKind`] for the [`AssetStorage`].
    fn kind(&self) -> AssetKind;
    /// Determine if an Asset exists in the storage for the [`AssetKey`].
    ///
    /// # Errors
    ///
    /// Will return Err if the data backing the [`AssetStorage`] is unreachable or busy.
    fn exists(&self, key: &AssetKey) -> miette::Result<bool>;
    /// Save an Asset to the [`AssetStorage`] using an [`AssetKey`].
    ///
    /// # Errors
    ///
    /// Will return Err if the data backing the [`AssetStorage`] is unreachable or busy.
    fn save(&self, key: &AssetKey, value: Value) -> miette::Result<()>;
    /// Load an Asset from the [`AssetStorage`] using an [`AssetKey`].
    ///
    /// # Errors
    ///
    /// Will return Err if the data backing the [`AssetStorage`] is unreachable or busy.
    fn load(&self, key: &AssetKey) -> miette::Result<Value>;
}
