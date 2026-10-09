/*!
This module defines the interface contracts that the various [`AssetStorage`]
implementations must satisfy.
*/
use std::{fmt::Display, path::PathBuf, str::FromStr, time::SystemTime};

use futures::future::BoxFuture;
use miette::{Context, IntoDiagnostic, miette};
use url::{Host, Url};
use uuid::Uuid;

/// [`AssetKind`] is used to categorize [`AssetSpec`] and [`AssetStorage`] implementations
/// as some [Executor][crate::executor::Executor] implementations may make use of this detail.
///
/// For instance the [`SubprocessExecutor`][crate::executor::SubprocessExecutor] struct requires that
/// Task inputs and outputs are of [`AssetKind::File`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AssetKind {
    /// An Asset that is stored in memory during Workflow execution.
    ///
    /// These Assets will be lost between runs or if the Workflow server
    /// crashes during execution.
    Memory,
    /// An asset that is stored in a file on disk during Workflow Execution.
    ///
    /// Contains a `root` file path that can be used to construct the full
    /// path to the asset in the filesystem.
    File {
        /// The root directory for constructing the full path to an Asset
        /// in the filesystem.
        root: PathBuf,
    },
    /// An asset that is accessible through a compliant implementation
    /// of the JSON:API specification: <https://jsonapi.org/>
    JsonAPI {
        /// The host name of the JSON:API compliant resource.
        host: Host,
        /// The resource type of the JSON:API resource.
        resource_type: String,
    },
}

impl FromStr for AssetKind {
    type Err = miette::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let url = Url::from_str(s).into_diagnostic()?;
        match url.scheme() {
            "memory" => Ok(Self::Memory),
            "file" => Ok(Self::File {
                root: url.path().parse().into_diagnostic().wrap_err_with(|| {
                    miette!("Failed to parse root folder location, for url: {url}")
                })?,
            }),
            "http+jsonapi" => {
                let host = url
                    .host()
                    .ok_or_else(|| miette!("No host specified for http+jsonapi AssetKind"))?;
                let resource_type = url.path().trim_matches('/').to_string();
                Ok(Self::JsonAPI {
                    host: host.to_owned(),
                    resource_type,
                })
            }
            scheme => Err(miette!("Unknown scheme: {scheme}")),
        }
    }
}

impl Display for AssetKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Memory => write!(f, "memory://process"),
            Self::File { root } => write!(f, "file://{}", root.to_string_lossy()),
            Self::JsonAPI {
                host,
                resource_type,
            } => write!(f, "http+jsonapi://{host}/{resource_type}"),
        }
    }
}

/// [`AssetSpec`] describes how an Asset should be stored.
///
/// The Asset may not have been persisted yet depending on the Workflow execution
/// and Executors may wish to reserve [`AssetSpec`]s for Tasks.
#[derive(Clone, Debug, PartialEq, Eq)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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

impl FromStr for AssetKey {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
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
    /// Reserve an [`AssetKey`] for this [`AssetStorage`].
    ///
    /// For most implementations this will just involve checking that an Asset does not
    /// already exist with that key.
    ///
    /// # Errors
    ///
    /// Will return Err if the data backing the [`AssetStorage`] is unreachable or busy.
    fn reserve(&self, key: &AssetKey) -> BoxFuture<'_, miette::Result<AssetKind>>;
    /// Save an Asset to the [`AssetStorage`] using an [`AssetKey`].
    ///
    /// # Errors
    ///
    /// Will return Err if the data backing the [`AssetStorage`] is unreachable or busy.
    fn save(&self, key: &AssetKey, value: Vec<u8>) -> BoxFuture<'_, miette::Result<AssetKind>>;
    /// Load an Asset from the [`AssetStorage`] using an [`AssetKey`].
    ///
    /// # Errors
    ///
    /// Will return Err if the data backing the [`AssetStorage`] is unreachable or busy.
    fn load(&self, key: &AssetKey) -> BoxFuture<'_, miette::Result<Vec<u8>>>;
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::memory(AssetKind::Memory, "memory://process")]
    #[case::empty_root(AssetKind::File { root: "/".parse().unwrap() }, "file:///")]
    #[case::tmp_dir(AssetKind::File { root: "/tmp".parse().unwrap() }, "file:///tmp")]
    #[case::tmp_dir(AssetKind::JsonAPI {
        host: Host::Domain("localhost".to_string()),
        resource_type: "comment".to_string(),
    }, "http+jsonapi://localhost/comment")]
    #[case::tmp_dir(AssetKind::JsonAPI {
        host: Host::Domain("nexus.quantinuum.com".to_string()),
        resource_type: "api/circuits/v1beta2".to_string(),
    }, "http+jsonapi://nexus.quantinuum.com/api/circuits/v1beta2")]
    fn test_asset_kind_to_from_str(
        #[case] asset_kind: AssetKind,
        #[case] expected_str: &str,
    ) -> miette::Result<()> {
        assert_eq!(asset_kind.to_string(), expected_str);
        let parsed: AssetKind = expected_str.parse()?;
        assert_eq!(parsed, asset_kind);
        Ok(())
    }
}
